using System.Text.Json;

namespace EtAlii.Adp.Parity.Recorder;

/// <summary>
/// One edit script, as contracts/parity.md shapes it: the tool type, the fixture it opens and the
/// steps it replays. Read leniently into steps the replay understands; anything it does not is
/// refused with the step's index, so a script never half-records.
/// </summary>
internal sealed record EditScript(
    string Tool,
    string Name,
    string Fixture,
    string? Registration,
    IReadOnlyList<Step> Steps,
    string SourcePath)
{
    /// <summary>The project-relative path the script opens: its registration when it has one, else its fixture.</summary>
    public string Document => Registration ?? Fixture;

    /// <summary>The folder under parity/fixtures/ that becomes the project: the one holding the document.</summary>
    public string FixtureFolder => Path.GetDirectoryName(Document.Replace('\\', '/'))!.Replace('\\', '/');

    /// <summary>The document's path relative to the project folder.</summary>
    public string DocumentInProject => Path.GetFileName(Document);

    /// <summary>How many steps change something: SC-004 counts these.</summary>
    public int EditCount => Steps.Sum(step => step switch
    {
        ActionStep or PropertyStep or SaveTextStep => 1,
        MoveStep move => move.Refs.Count,
        UndoStep undo => undo.Count,
        RedoStep redo => redo.Count,
        _ => 0,
    });

    public static EditScript Load(string path, string scriptsRoot)
    {
        using var json = JsonDocument.Parse(File.ReadAllBytes(path), new JsonDocumentOptions { CommentHandling = JsonCommentHandling.Skip, AllowTrailingCommas = true });
        var root = json.RootElement;
        var tool = Required(root, "tool", path);
        var fixture = Required(root, "fixture", path);
        var registration = root.TryGetProperty("registration", out var reg) && reg.ValueKind == JsonValueKind.String ? reg.GetString() : null;

        // The golden folder is named after the script's path below parity/scripts/<tool>/.
        var relative = Path.GetRelativePath(scriptsRoot, path).Replace('\\', '/');
        var expectedPrefix = tool + "/";
        if (!relative.StartsWith(expectedPrefix, StringComparison.Ordinal))
        {
            throw new ScriptException(path, $"lies at '{relative}' but names the tool '{tool}'; scripts live under parity/scripts/<tool>/<name>.json");
        }

        var name = relative[expectedPrefix.Length..];
        name = name[..^Path.GetExtension(name).Length];

        if (registration is not null && Path.GetDirectoryName(registration) != Path.GetDirectoryName(fixture))
        {
            throw new ScriptException(path, "the fixture and its registration must be in one folder");
        }

        if (!root.TryGetProperty("steps", out var steps) || steps.ValueKind != JsonValueKind.Array)
        {
            throw new ScriptException(path, "has no steps array");
        }

        var parsed = new List<Step>();
        var index = 0;
        foreach (var step in steps.EnumerateArray())
        {
            parsed.Add(Step.Parse(step, index, path));
            index++;
        }

        return new EditScript(tool, name, fixture, registration, parsed, path);
    }

    private static string Required(JsonElement root, string property, string path) =>
        root.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.String && value.GetString() is { Length: > 0 } text
            ? text
            : throw new ScriptException(path, $"has no '{property}'");
}

internal sealed class ScriptException(string path, string message) : Exception($"{path}: {message}");

/// <summary>What a prompt is answered with: the value SubmitInteraction sends, and the dialog's text field.</summary>
internal sealed record Answer(string Value, string Text);

/// <summary>One step of a script. Every step may say it expects the standalone host to refuse it.</summary>
internal abstract record Step(int Index, bool ExpectRefusal)
{
    public static Step Parse(JsonElement step, int index, string path)
    {
        if (step.ValueKind != JsonValueKind.Object)
        {
            throw new ScriptException(path, $"step {index} is not an object");
        }

        var refused = step.TryGetProperty("expect", out var expect) && expect.ValueKind == JsonValueKind.String && expect.GetString() == "refused";

        if (step.TryGetProperty("select", out var select))
        {
            return new SelectStep(index, refused, Strings(select, index, path));
        }

        if (step.TryGetProperty("action", out var action))
        {
            var args = step.TryGetProperty("args", out var a) && a.ValueKind == JsonValueKind.Object ? a : default;
            string? target = args.ValueKind == JsonValueKind.Object && args.TryGetProperty("target", out var t) ? t.GetString() : null;
            var answers = new List<Answer>();
            if (args.ValueKind == JsonValueKind.Object)
            {
                if (args.TryGetProperty("answers", out var list) && list.ValueKind == JsonValueKind.Array)
                {
                    answers.AddRange(list.EnumerateArray().Select(answer => ParseAnswer(answer, index, path)));
                }
                else if (args.TryGetProperty("value", out _) || args.TryGetProperty("text", out _) || args.TryGetProperty("confirm", out _))
                {
                    answers.Add(ParseAnswer(args, index, path));
                }
            }

            return new ActionStep(index, refused, action.GetString() ?? throw new ScriptException(path, $"step {index} has no action id"), target, answers);
        }

        if (step.TryGetProperty("property", out var property))
        {
            var row = property.TryGetProperty("row", out var r) ? r.GetString() : null;
            var value = property.TryGetProperty("value", out var v) ? v.GetString() : null;
            var target = property.TryGetProperty("target", out var t) ? t.GetString() : null;
            if (row is null || value is null)
            {
                throw new ScriptException(path, $"step {index}: a property step needs a row and a value");
            }

            return new PropertyStep(index, refused, row, value, target);
        }

        if (step.TryGetProperty("move", out var move))
        {
            var refs = move.TryGetProperty("refs", out var r) ? Strings(r, index, path) : throw new ScriptException(path, $"step {index}: a move needs refs");
            if (move.TryGetProperty("parent", out var parent))
            {
                var at = move.TryGetProperty("index", out var i) ? i.GetInt32() : -1;
                return new MoveStep(index, refused, refs, MoveKind.Reparent, parent.GetString(), at, 0, 0);
            }

            if (move.TryGetProperty("to", out var to))
            {
                return new MoveStep(index, refused, refs, MoveKind.To, null, -1, to.GetProperty("x").GetDouble(), to.GetProperty("y").GetDouble());
            }

            var dx = move.TryGetProperty("dx", out var x) ? x.GetDouble() : 0;
            var dy = move.TryGetProperty("dy", out var y) ? y.GetDouble() : 0;
            return new MoveStep(index, refused, refs, MoveKind.By, null, -1, dx, dy);
        }

        if (step.TryGetProperty("undo", out var undo))
        {
            return new UndoStep(index, refused, undo.GetInt32());
        }

        if (step.TryGetProperty("redo", out var redo))
        {
            return new RedoStep(index, refused, redo.GetInt32());
        }

        if (step.TryGetProperty("save", out var save))
        {
            if (save.ValueKind == JsonValueKind.Object)
            {
                var content = save.TryGetProperty("content", out var c) ? c.GetString() : null;
                var file = save.TryGetProperty("path", out var p) ? p.GetString() : null;
                return content is null
                    ? throw new ScriptException(path, $"step {index}: a save of text needs its content")
                    : new SaveTextStep(index, refused, file, content);
            }

            return new SaveStep(index, refused);
        }

        throw new ScriptException(path, $"step {index} is none of select, action, property, move, undo, redo or save");
    }

    private static Answer ParseAnswer(JsonElement answer, int index, string path)
    {
        if (answer.ValueKind == JsonValueKind.String)
        {
            return new Answer(answer.GetString()!, "");
        }

        if (answer.ValueKind != JsonValueKind.Object)
        {
            throw new ScriptException(path, $"step {index}: an answer is a string or an object");
        }

        var value = answer.TryGetProperty("value", out var v) ? v.GetString() ?? "" : "";
        var text = answer.TryGetProperty("text", out var t) ? t.GetString() ?? "" : "";
        return new Answer(value, text);
    }

    private static List<string> Strings(JsonElement value, int index, string path) =>
        value.ValueKind switch
        {
            JsonValueKind.Array => value.EnumerateArray().Select(item => item.GetString() ?? "").ToList(),
            JsonValueKind.String => [value.GetString()!],
            _ => throw new ScriptException(path, $"step {index}: expected an id or a list of ids"),
        };
}

internal sealed record SelectStep(int Index, bool ExpectRefusal, IReadOnlyList<string> Ids) : Step(Index, ExpectRefusal);

internal sealed record ActionStep(int Index, bool ExpectRefusal, string ActionId, string? Target, IReadOnlyList<Answer> Answers) : Step(Index, ExpectRefusal);

internal sealed record PropertyStep(int Index, bool ExpectRefusal, string Row, string Value, string? Target) : Step(Index, ExpectRefusal);

internal enum MoveKind
{
    Reparent,
    To,
    By,
}

internal sealed record MoveStep(int Index, bool ExpectRefusal, IReadOnlyList<string> Refs, MoveKind Kind, string? Parent, int At, double X, double Y) : Step(Index, ExpectRefusal);

internal sealed record UndoStep(int Index, bool ExpectRefusal, int Count) : Step(Index, ExpectRefusal);

internal sealed record RedoStep(int Index, bool ExpectRefusal, int Count) : Step(Index, ExpectRefusal);

internal sealed record SaveStep(int Index, bool ExpectRefusal) : Step(Index, ExpectRefusal);

internal sealed record SaveTextStep(int Index, bool ExpectRefusal, string? File, string Content) : Step(Index, ExpectRefusal);

using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Channels;
using EtAlii.Adp.Authentication.Wire;
using EtAlii.Adp.Context.Wire;
using EtAlii.Adp.Diagram.Wire;
using EtAlii.Adp.Documents.Wire;
using EtAlii.Adp.Editor.Wire;
using EtAlii.Adp.Hierarchy.Wire;
using EtAlii.Adp.Projects.Wire;
using EtAlii.Adp.TestSupport.Host;
using Grpc.Core;
using Grpc.Net.Client;
using Path = EtAlii.Adp.Documents.Wire.Path;
using WireShortGuid = EtAlii.Adp.Documents.Wire.ShortGuid;

namespace EtAlii.Adp.Parity.Recorder;

/// <summary>An element id the standalone host generated during a replay: the step that made it and what it is.</summary>
internal sealed record GeneratedId(int Step, string Id, string Type, bool Connection);

/// <summary>What one step did, as recorded.json lists it.</summary>
internal sealed record StepRecord(int Step, string Kind, string Rpc, string Outcome, string Detail);

/// <summary>What a replay leaves: the project folder after the last step, and what happened on the way.</summary>
internal sealed record ReplayResult(IReadOnlyList<StepRecord> Steps, IReadOnlyList<GeneratedId> Generated, IReadOnlyList<string> Notices);

/// <summary>
/// One replay of one script against one in-process standalone host: signed in with the developer
/// session, the fixture added as a project, the document open on one connection, every step sent
/// as the client would send it.
/// </summary>
internal sealed partial class Replay : IAsyncDisposable
{
    private static readonly TimeSpan PromptTimeout = TimeSpan.FromSeconds(20);
    private static readonly TimeSpan NoPromptPatience = TimeSpan.FromMilliseconds(750);
    private static readonly TimeSpan Quiet = TimeSpan.FromMilliseconds(300);
    private static readonly TimeSpan QuietLimit = TimeSpan.FromSeconds(10);
    private static readonly TimeSpan Settle = TimeSpan.FromMilliseconds(750);

    private readonly EditScript _script;
    private readonly string _projectFolder;
    private readonly string _initialText;
    private readonly GrpcChannel _channel;
    private readonly CancellationTokenSource _streams = new();

    private readonly ContextService.ContextServiceClient _context;
    private readonly DiagramService.DiagramServiceClient _diagrams;
    private readonly EditorService.EditorServiceClient _editors;
    private readonly HierarchyService.HierarchyServiceClient _hierarchy;

    private readonly Channel<ContextPrompt> _prompts = Channel.CreateUnbounded<ContextPrompt>();
    private readonly List<string> _notices = [];
    private readonly List<StepRecord> _steps = [];

    // What the diagram stream has said, kept by the stream's reader and read between steps.
    private readonly Lock _elementsLock = new();
    private readonly Dictionary<string, Element> _elements = new(StringComparer.Ordinal);
    private readonly HashSet<string> _seen = new(StringComparer.Ordinal);
    private readonly List<GeneratedId> _generated = [];
    private long _lastDeltaTicks;
    private int _currentStep = -1;

    private Metadata _headers = new();
    private WireShortGuid _projectId = new();
    private readonly WireShortGuid _watchId = ShortGuid.NewShortGuid();
    private WireShortGuid _entryId = new();
    private Path _documentPath = new();
    private AsyncServerStreamingCall<ContextMessage>? _contextCall;
    private AsyncServerStreamingCall<Delta>? _diagramCall;
    private Task? _contextReader;
    private Task? _diagramReader;

    private Replay(EditScript script, string projectFolder, string initialText, GrpcChannel channel)
    {
        _script = script;
        _projectFolder = projectFolder;
        _initialText = initialText;
        _channel = channel;
        _context = new ContextService.ContextServiceClient(channel);
        _diagrams = new DiagramService.DiagramServiceClient(channel);
        _editors = new EditorService.EditorServiceClient(channel);
        _hierarchy = new HierarchyService.HierarchyServiceClient(channel);
    }

    public static async Task<ReplayResult> RunAsync(HostFixture host, EditScript script, string projectFolder, string initialText, TextWriter? elements, CancellationToken cancellationToken)
    {
        await using var replay = new Replay(script, projectFolder, initialText, host.CreateChannel());
        await replay.OpenAsync(cancellationToken);
        if (elements is not null)
        {
            // For whoever writes a script: what the document shows when it opens, as the stream says it.
            lock (replay._elementsLock)
            {
                foreach (var element in replay._elements.Values.OrderBy(element => element.Id.Value, StringComparer.Ordinal))
                {
                    var position = element.Position is { } p ? $"({p.X}, {p.Y})" : "-";
                    var ends = element.SourceId is not null ? $" {element.SourceId.Value} -> {element.TargetId?.Value}" : "";
                    elements.WriteLine($"  element {element.Id.Value} {element.Type} at {position}{ends}");
                }
            }
        }

        foreach (var step in script.Steps)
        {
            await replay.RunStepAsync(step, cancellationToken);
        }

        await replay.WaitUntilQuietAsync(cancellationToken);
        return new ReplayResult(replay._steps, replay.GeneratedSnapshot(), replay._notices);
    }

    // ---- opening -------------------------------------------------------------------------------

    private async Task OpenAsync(CancellationToken cancellationToken)
    {
        // The developer session: a real session for the configured developer identity, issued
        // only by a Debug build running in the developer environment.
        var authentication = new AuthenticationService.AuthenticationServiceClient(_channel);
        var session = await authentication.DeveloperSessionAsync(new DeveloperSessionRequest(), cancellationToken: cancellationToken);
        if (session.ResultCase != DeveloperSessionResponse.ResultOneofCase.Session || !session.Bypassed)
        {
            throw new InvalidOperationException(
                "The standalone host issued no developer session. The recorder must be built in Debug (the default of `dotnet run`), " +
                "because the developer session is compiled into a Debug build of the standalone host only.");
        }

        _headers = new Metadata { { FlowConstants.SessionTokenHeader, session.Session.Value } };

        var projects = new ProjectService.ProjectServiceClient(_channel);
        var added = await projects.AddProjectAsync(new AddProjectRequest { Path = Path.OfLocation(_projectFolder) }, _headers, cancellationToken: cancellationToken);
        if (added.ResultCase != AddProjectResponse.ResultOneofCase.Added)
        {
            throw new InvalidOperationException($"The standalone host refused the project {_projectFolder}: {added.Error?.Message}");
        }

        _projectId = added.Added.Id;
        if (IsTable)
        {
            // A designer's document is a table on the tab's one stream, not a diagram.
            await OpenTableAsync(cancellationToken);
            return;
        }

        // The context watch first: prompts are pushed on it, and an action is refused on a
        // connection that is not watching. Its first message is the proof it is registered.
        _contextCall = _context.Watch(new WatchContextRequest { ProjectId = _projectId, WatchId = _watchId }, _headers, cancellationToken: _streams.Token);
        using (var first = Timeout(cancellationToken, PromptTimeout))
        {
            if (!await _contextCall.ResponseStream.MoveNext(first.Token))
            {
                throw new InvalidOperationException("The context watch ended before its baseline.");
            }
        }

        _contextReader = Task.Run(ReadContextAsync, CancellationToken.None);

        _entryId = await EntryIdAsync(_script.DocumentInProject, cancellationToken);
        _documentPath = Path.Of(_script.DocumentInProject);

        // The document open on this very connection, as a tab is: a move is refused for a diagram
        // the connection does not have open, and the stream is where generated ids are seen.
        _diagramCall = _diagrams.Open(
            new OpenDiagramRequest { ProjectId = _projectId, WatchId = _watchId, Path = _documentPath },
            _headers,
            cancellationToken: _streams.Token);
        var baseline = new TaskCompletionSource();
        _diagramReader = Task.Run(() => ReadDiagramAsync(baseline), CancellationToken.None);
        using (var first = Timeout(cancellationToken, PromptTimeout))
        {
            await baseline.Task.WaitAsync(first.Token);
        }

        // Everything in view, so every element reaches the stream whatever the module's viewport.
        var view = await _diagrams.UpdateViewAsync(
            new UpdateViewRequest
            {
                ProjectId = _projectId,
                WatchId = _watchId,
                Path = _documentPath,
                View = new ViewUpdate
                {
                    Center = new Point2D(),
                    BoundingBox = new BoundingBox { Min = new Point2D { X = -1e13, Y = -1e13 }, Max = new Point2D { X = 1e13, Y = 1e13 } },
                },
            },
            _headers,
            cancellationToken: cancellationToken);
        if (view.Error.Length > 0)
        {
            throw new InvalidOperationException($"The standalone host refused the view of {_script.DocumentInProject}: {view.Error}");
        }

        // The tab's own selection: the document, nothing inside it yet.
        await SelectAsync([], cancellationToken);
        await WaitUntilQuietAsync(cancellationToken);
    }

    private async Task<WireShortGuid> EntryIdAsync(string name, CancellationToken cancellationToken)
    {
        // Breadth first through folders and through the files registrations nest under.
        var pending = new Queue<WireShortGuid?>();
        pending.Enqueue(null);
        while (pending.Count > 0)
        {
            var folder = pending.Dequeue();
            var request = new ListEntriesRequest { ProjectId = _projectId, WatchId = _watchId };
            if (folder is not null)
            {
                request.FolderId = folder;
            }

            var response = await _hierarchy.ListEntriesAsync(request, _headers, cancellationToken: cancellationToken);
            if (response.ResultCase != ListEntriesResponse.ResultOneofCase.Entries)
            {
                throw new InvalidOperationException($"The standalone host could not list the project: {response.Error?.Message}");
            }

            foreach (var entry in response.Entries.Entries_)
            {
                if (entry.Name == name)
                {
                    return entry.Id;
                }

                if (entry.HasChildren)
                {
                    pending.Enqueue(entry.Id);
                }
            }
        }

        throw new InvalidOperationException($"The project has no entry named {name}.");
    }

    // ---- the streams ---------------------------------------------------------------------------

    private async Task ReadContextAsync()
    {
        try
        {
            while (await _contextCall!.ResponseStream.MoveNext(_streams.Token))
            {
                var message = _contextCall.ResponseStream.Current;
                switch (message.MessageCase)
                {
                    case ContextMessage.MessageOneofCase.Prompt:
                        await _prompts.Writer.WriteAsync(message.Prompt, _streams.Token);
                        break;
                    case ContextMessage.MessageOneofCase.Notice:
                        lock (_notices)
                        {
                            _notices.Add($"step {_currentStep}: {message.Notice.Message}");
                        }

                        break;
                }
            }
        }
        catch (Exception exception) when (exception is OperationCanceledException or RpcException { StatusCode: StatusCode.Cancelled })
        {
            // Closed at the end of the replay.
        }
    }

    private async Task ReadDiagramAsync(TaskCompletionSource baseline)
    {
        try
        {
            while (await _diagramCall!.ResponseStream.MoveNext(_streams.Token))
            {
                var delta = _diagramCall.ResponseStream.Current;
                lock (_elementsLock)
                {
                    // Everything the stream says while the document is opening is its baseline.
                    Apply(delta, isBaseline: Volatile.Read(ref _currentStep) < 0);
                    Interlocked.Exchange(ref _lastDeltaTicks, DateTime.UtcNow.Ticks);
                }

                if (delta.ActionCase == Delta.ActionOneofCase.Add)
                {
                    baseline.TrySetResult();
                }
            }

            baseline.TrySetException(new InvalidOperationException("The diagram stream ended before its baseline."));
        }
        catch (Exception exception) when (exception is OperationCanceledException or RpcException { StatusCode: StatusCode.Cancelled })
        {
            baseline.TrySetCanceled();
        }
        catch (RpcException exception)
        {
            baseline.TrySetException(new InvalidOperationException($"The standalone host refused to open {_script.DocumentInProject}: {exception.Status.Detail}", exception));
        }
    }

    private void Apply(Delta delta, bool isBaseline)
    {
        switch (delta.ActionCase)
        {
            case Delta.ActionOneofCase.Add:
                foreach (var element in delta.Add.Elements)
                {
                    Track(element, isBaseline);
                }

                break;
            case Delta.ActionOneofCase.Remove:
                foreach (var id in delta.Remove.ElementIds)
                {
                    _elements.Remove(id.Value);
                }

                break;
            case Delta.ActionOneofCase.Group:
                foreach (var id in delta.Group.SourceElementIds)
                {
                    _elements.Remove(id.Value);
                }

                // A fold's stand-in is the module's own and is never written to a file.
                _seen.Add(delta.Group.GroupElement.Id.Value);
                _elements[delta.Group.GroupElement.Id.Value] = delta.Group.GroupElement;
                break;
            case Delta.ActionOneofCase.Ungroup:
                _elements.Remove(delta.Ungroup.GroupElementId.Value);
                foreach (var element in delta.Ungroup.Elements)
                {
                    Track(element, isBaseline: true);
                }

                break;
        }
    }

    private void Track(Element element, bool isBaseline)
    {
        var id = element.Id.Value;
        _elements[id] = element;

        // Generated: never on the stream before, and in none of the fixture's files - an element
        // a fold hid, or one an undo brings back, is neither.
        if (_seen.Add(id) && !isBaseline && !_initialText.Contains(id, StringComparison.Ordinal))
        {
            _generated.Add(new GeneratedId(_currentStep, id, element.Type, IsConnection(element)));
        }
    }

    /// <summary>
    /// Whether an element is a connection: one sent with its two ends, or - for a module that has
    /// not moved its connections to the shaped fields yet - one whose type says so.
    /// </summary>
    private static bool IsConnection(Element element) =>
        element.SourceId is not null
        || element.Type.EndsWith("+connection", StringComparison.Ordinal)
        || element.Type.EndsWith("+relationship", StringComparison.Ordinal)
        || element.Type.EndsWith("+relation", StringComparison.Ordinal);

    private List<GeneratedId> GeneratedSnapshot()
    {
        lock (_elementsLock)
        {
            return [.. _generated];
        }
    }

    private async Task WaitUntilQuietAsync(CancellationToken cancellationToken)
    {
        var started = DateTime.UtcNow;
        while (DateTime.UtcNow - started < QuietLimit)
        {
            var last = new DateTime(Interlocked.Read(ref _lastDeltaTicks), DateTimeKind.Utc);
            var quietFor = DateTime.UtcNow - last;
            if (quietFor >= Quiet)
            {
                return;
            }

            await Task.Delay(Quiet - quietFor, cancellationToken);
        }
    }

    // ---- steps ---------------------------------------------------------------------------------

    private async Task RunStepAsync(Step step, CancellationToken cancellationToken)
    {
        Volatile.Write(ref _currentStep, step.Index);

        // The explorer's entry for the document is renewed when the standalone host replaces the
        // file on disk; a client hears of that on its hierarchy stream and names the new entry from
        // then on, which is what looking it up again before every step does here.
        _entryId = await EntryIdAsync(_script.DocumentInProject, cancellationToken);
        var (rpc, refusal, detail) = step switch
        {
            SelectStep select => await RunSelectAsync(select, cancellationToken),
            ActionStep action => await RunActionAsync(action, cancellationToken),
            PropertyStep property => await RunPropertyAsync(property, cancellationToken),
            MoveStep move => await RunMoveAsync(move, cancellationToken),
            UndoStep undo => await RunHistoryAsync("history.undo", undo.Count, cancellationToken),
            RedoStep redo => await RunHistoryAsync("history.redo", redo.Count, cancellationToken),
            SaveStep => ("none", null, "every command is written by the history as it runs; nothing is sent"),
            SaveTextStep save => await RunSaveTextAsync(save, cancellationToken),
            TableStep table when IsTable => await RunTableAsync(table, cancellationToken),
            ViewStep view when IsTable => await RunViewAsync(view, cancellationToken),
            _ => throw new InvalidOperationException($"Step {step.Index} has no replay."),
        };

        await WaitUntilQuietAsync(cancellationToken);
        if (IsTable)
        {
            TrackGeneratedInFiles();
        }

        // The standalone host validates a changed file in the background, reading it as it goes. A
        // write that lands while a reader holds the file can fail on Windows (File.Replace,
        // 0x80070497), so the next step waits for that pass to end, as a person's next edit would.
        await Task.Delay(Settle, cancellationToken);

        var kind = step.GetType().Name.Replace("Step", "", StringComparison.Ordinal).ToLowerInvariant();
        if (refusal is not null && !step.ExpectRefusal)
        {
            throw new ScriptException(_script.SourcePath, $"step {step.Index} ({kind}) was refused by the standalone host: {refusal}");
        }

        if (refusal is null && step.ExpectRefusal)
        {
            throw new ScriptException(_script.SourcePath, $"step {step.Index} ({kind}) expected a refusal, but the standalone host accepted it");
        }

        _steps.Add(new StepRecord(step.Index, kind, rpc, refusal is null ? "accepted" : "refused", refusal ?? detail));
    }

    private async Task<(string, string?, string)> RunSelectAsync(SelectStep step, CancellationToken cancellationToken)
    {
        if (step.Ids.Count > 1)
        {
            throw new ScriptException(_script.SourcePath, $"step {step.Index}: ContextService.Select carries one element; select one id at a time");
        }

        var ids = step.Ids.Select(Resolve).ToList();
        var error = await SelectAsync(ids, cancellationToken);

        return ("ContextService.Select", error.Length > 0 ? error : null, ids.Count == 0 ? "the document" : ids[0]);
    }

    private async Task<string> SelectAsync(IReadOnlyList<string> ids, CancellationToken cancellationToken)
    {
        var selection = new ContextSelection
        {
            Source = ContextSelectionSource.Explorer,
            Id = new ContextSource { EntryId = _entryId },
            Path = new Path(),
        };
        if (ids.Count == 1)
        {
            selection.Child = new ContextSelection
            {
                Source = ContextSelectionSource.DiagramCanvas,
                Id = new ContextSource { ElementId = new ElementId { Value = ids[0] } },
                Path = new Path(),
            };
        }

        var response = await _context.SelectAsync(
            new SelectRequest { ProjectId = _projectId, WatchId = _watchId, Selection = selection },
            _headers,
            cancellationToken: cancellationToken);
        return response.Error;
    }

    private async Task<(string, string?, string)> RunActionAsync(ActionStep step, CancellationToken cancellationToken)
    {
        var interactionId = ShortGuid.NewShortGuid();
        var request = new ExecuteActionRequest
        {
            ProjectId = _projectId,
            WatchId = _watchId,
            InteractionId = interactionId,
            ActionId = step.ActionId,
        };
        if (step.Target is not null)
        {
            request.Source = new ContextSource { ElementId = new ElementId { Value = Resolve(step.Target) }, DiagramEntryId = _entryId };
        }
        else if (step.Entry is not null)
        {
            // An explorer entry by its name - a folder an add creates in, a file an add registers - as its
            // context menu sends it.
            request.Source = new ContextSource { EntryId = await EntryIdAsync(step.Entry, cancellationToken) };
        }
        else if (step.ActionId.StartsWith("history.", StringComparison.Ordinal))
        {
            request.Source = new ContextSource { Project = new Google.Protobuf.WellKnownTypes.Empty() };
        }

        // Prompts that arrived for nobody before this action are not this action's.
        while (_prompts.Reader.TryRead(out _))
        {
        }

        var executed = await _context.ExecuteActionAsync(request, _headers, cancellationToken: cancellationToken);
        if (!executed.Accepted)
        {
            return ("ContextService.ExecuteAction", executed.Error.Length > 0 ? executed.Error : "not accepted", "");
        }

        var prompt = await NextPromptAsync(interactionId, step.Answers.Count > 0 ? PromptTimeout : NoPromptPatience, cancellationToken);
        var log = new StringBuilder(step.Target is not null ? $"on {request.Source.ElementId.Value}" : step.Entry is not null ? $"on the entry {step.Entry}" : "on the selection");
        var rpc = "ContextService.ExecuteAction";
        foreach (var answer in step.Answers)
        {
            if (prompt is null)
            {
                return (rpc, $"answer '{answer.Value}' given, but no prompt was shown", log.ToString());
            }

            log.Append($"; {Describe(prompt)} answered '{answer.Value}'");
            rpc += " + SubmitInteraction";
            var submitted = await _context.SubmitInteractionAsync(
                new SubmitInteractionRequest { InteractionId = prompt.InteractionId, Value = answer.Value, Text = answer.Text },
                _headers,
                cancellationToken: cancellationToken);
            if (!submitted.Completed)
            {
                return (rpc, submitted.Error.Length > 0 ? submitted.Error : "not completed", log.ToString());
            }

            prompt = submitted.NextPrompt is { PromptCase: not ContextPrompt.PromptOneofCase.None and not ContextPrompt.PromptOneofCase.Closed } next ? next : null;
        }

        if (prompt is not null)
        {
            // An unanswered prompt is dismissed, as Escape dismisses it; what the action did before
            // asking - an add that then asks for a name - stays.
            log.Append($"; {Describe(prompt)} dismissed");
            rpc += " + CancelInteraction";
            await _context.CancelInteractionAsync(new CancelInteractionRequest { InteractionId = prompt.InteractionId }, _headers, cancellationToken: cancellationToken);
        }

        return (rpc, null, log.ToString());
    }

    private async Task<ContextPrompt?> NextPromptAsync(WireShortGuid interactionId, TimeSpan patience, CancellationToken cancellationToken)
    {
        using var wait = Timeout(cancellationToken, patience);
        try
        {
            while (true)
            {
                var prompt = await _prompts.Reader.ReadAsync(wait.Token);
                if (prompt.InteractionId.Equals(interactionId) && prompt.PromptCase is not ContextPrompt.PromptOneofCase.Closed)
                {
                    return prompt;
                }
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return null;
        }
    }

    private static string Describe(ContextPrompt prompt) => prompt.PromptCase switch
    {
        ContextPrompt.PromptOneofCase.InputDialog => $"input '{prompt.InputDialog.Title}'",
        ContextPrompt.PromptOneofCase.ConfirmDialog => $"confirmation '{prompt.ConfirmDialog.Title}'",
        ContextPrompt.PromptOneofCase.ChoiceDialog => $"choice '{prompt.ChoiceDialog.Title}'",
        ContextPrompt.PromptOneofCase.FileDialog => $"file '{prompt.FileDialog.Title}'",
        _ => prompt.PromptCase.ToString(),
    };

    private async Task<(string, string?, string)> RunPropertyAsync(PropertyStep step, CancellationToken cancellationToken)
    {
        ContextSource? source = step.Target is null
            ? null
            : new ContextSource { ElementId = new ElementId { Value = Resolve(step.Target) }, DiagramEntryId = _entryId };

        // A row is named by its property id or, as the grid shows it, by its label.
        var propertyId = step.Row;
        if (!step.Row.Contains('.', StringComparison.Ordinal))
        {
            var describeRequest = new DescribePropertiesRequest { ProjectId = _projectId, WatchId = _watchId };
            if (source is not null)
            {
                describeRequest.Source = source;
            }

            var described = await _context.DescribePropertiesAsync(describeRequest, _headers, cancellationToken: cancellationToken);
            var row = described.Properties.FirstOrDefault(property => string.Equals(property.Label, step.Row, StringComparison.OrdinalIgnoreCase));
            if (row is null)
            {
                return ("ContextService.DescribeProperties", $"no row labelled '{step.Row}' (rows: {string.Join(", ", described.Properties.Select(p => $"{p.Label}={p.Id}"))})", "");
            }

            propertyId = row.Id;
        }

        var request = new SetPropertyRequest { ProjectId = _projectId, WatchId = _watchId, PropertyId = propertyId, Value = step.Value };
        if (source is not null)
        {
            request.Source = source;
        }

        var response = await _context.SetPropertyAsync(request, _headers, cancellationToken: cancellationToken);
        return ("ContextService.SetProperty", response.Accepted ? null : response.Error.Length > 0 ? response.Error : "not accepted", $"{propertyId} = '{step.Value}'");
    }

    private async Task<(string, string?, string)> RunMoveAsync(MoveStep step, CancellationToken cancellationToken)
    {
        var details = new List<string>();
        foreach (var reference in step.Refs)
        {
            var id = Resolve(reference);
            var request = new MoveElementRequest { ProjectId = _projectId, WatchId = _watchId, Path = _documentPath, ElementId = id };
            switch (step.Kind)
            {
                case MoveKind.Reparent:
                    request.NewParentId = Resolve(step.Parent!);
                    request.Index = step.At;
                    details.Add($"{id} under {request.NewParentId} at {step.At}");
                    break;
                case MoveKind.To:
                    request.Position = new Point2D { X = step.X, Y = step.Y };
                    details.Add($"{id} to ({step.X}, {step.Y})");
                    break;
                case MoveKind.By:
                    Element? element;
                    lock (_elementsLock)
                    {
                        _elements.TryGetValue(id, out element);
                    }

                    if (element?.Position is null)
                    {
                        return ("DiagramService.MoveElement", $"{id} has no position on the stream to move from", "");
                    }

                    request.Position = new Point2D { X = element.Position.X + step.X, Y = element.Position.Y + step.Y };
                    details.Add($"{id} by ({step.X}, {step.Y}) to ({request.Position.X}, {request.Position.Y})");
                    break;
            }

            var response = await _diagrams.MoveElementAsync(request, _headers, cancellationToken: cancellationToken);
            if (response.Error.Length > 0)
            {
                return ("DiagramService.MoveElement", response.Error, string.Join("; ", details));
            }

            await WaitUntilQuietAsync(cancellationToken);
        }

        return ("DiagramService.MoveElement", null, string.Join("; ", details));
    }

    private async Task<(string, string?, string)> RunHistoryAsync(string actionId, int count, CancellationToken cancellationToken)
    {
        for (var i = 0; i < count; i++)
        {
            var response = await _context.ExecuteActionAsync(
                new ExecuteActionRequest
                {
                    ProjectId = _projectId,
                    WatchId = _watchId,
                    InteractionId = ShortGuid.NewShortGuid(),
                    ActionId = actionId,
                    Source = new ContextSource { Project = new Google.Protobuf.WellKnownTypes.Empty() },
                },
                _headers,
                cancellationToken: cancellationToken);
            if (!response.Accepted)
            {
                return ($"ContextService.ExecuteAction({actionId})", response.Error.Length > 0 ? response.Error : "not accepted", $"{i} of {count} done");
            }

            await WaitUntilQuietAsync(cancellationToken);
        }

        return ($"ContextService.ExecuteAction({actionId})", null, $"{count} time(s)");
    }

    private async Task<(string, string?, string)> RunSaveTextAsync(SaveTextStep step, CancellationToken cancellationToken)
    {
        var path = step.File is null ? _documentPath : Path.Of(step.File.Split('/'));
        var response = await _editors.SaveTextAsync(
            new SaveTextRequest { ProjectId = _projectId, WatchId = _watchId, Path = path, Content = step.Content },
            _headers,
            cancellationToken: cancellationToken);
        return ("EditorService.SaveText", response.Error.Length > 0 ? response.Error : null, string.Join('/', path.Segments));
    }

    // ---- references to generated ids -----------------------------------------------------------

    [GeneratedRegex(@"\$(c?)(\d+)")]
    private static partial Regex GeneratedReference();

    /// <summary>
    /// An id as the script names it, with <c>$N</c> replaced by the N-th element (counting from 1)
    /// and <c>$cN</c> by the N-th connection the standalone host generated so far in this replay.
    /// </summary>
    private string Resolve(string reference) =>
        GeneratedReference().Replace(reference, match =>
        {
            var connection = match.Groups[1].Value.Length > 0;
            var n = int.Parse(match.Groups[2].Value, System.Globalization.CultureInfo.InvariantCulture);
            var candidates = GeneratedSnapshot().Where(generated => generated.Connection == connection).ToList();
            return n >= 1 && n <= candidates.Count
                ? candidates[n - 1].Id
                : throw new ScriptException(_script.SourcePath, $"'{match.Value}' names generated {(connection ? "connection" : "element")} {n}, but only {candidates.Count} were generated so far");
        });

    private static CancellationTokenSource Timeout(CancellationToken cancellationToken, TimeSpan after)
    {
        var source = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        source.CancelAfter(after);
        return source;
    }

    public async ValueTask DisposeAsync()
    {
        await _streams.CancelAsync();
        foreach (var reader in new[] { _contextReader, _diagramReader })
        {
            if (reader is not null)
            {
                try
                {
                    await reader;
                }
                catch (Exception exception) when (exception is OperationCanceledException or RpcException)
                {
                    // Ended with the replay.
                }
            }
        }

        await DisposeTableAsync();
        _contextCall?.Dispose();
        _diagramCall?.Dispose();
        _channel.Dispose();
        _streams.Dispose();
    }
}

using System.Diagnostics;
using System.Reflection;
using System.Text.Encodings.Web;
using System.Text.Json;
using EtAlii.Adp.Parity.Recorder;
using EtAlii.Adp.TestSupport.Host;

// The parity recorder (etalii.adp spec 030, contracts/parity.md): replays edit scripts through the
// standalone host's gRPC API and writes what the standalone host wrote as the goldens.
//
//   dotnet run --project parity/recorder -- [script.json | folder ...] [--allow-dirty] [--any-commit] [--verbose] [--elements]
//
// Without a script or folder it records every script under parity/scripts/.

const string referenceCommit = "8b6ccfca4f4a65e43c1a1a73a3224abe7ebf800e";

var options = args.Where(arg => arg.StartsWith("--", StringComparison.Ordinal)).ToHashSet(StringComparer.Ordinal);
var targets = args.Where(arg => !arg.StartsWith("--", StringComparison.Ordinal)).ToList();
var unknown = options.Except(["--allow-dirty", "--any-commit", "--verbose", "--elements"]).ToList();
if (unknown.Count > 0)
{
    Console.Error.WriteLine($"Unknown option(s): {string.Join(", ", unknown)}");
    return 2;
}

var assembly = typeof(EditScript).Assembly;
var standaloneRoot = assembly.GetCustomAttributes<AssemblyMetadataAttribute>().Single(a => a.Key == "StandaloneRoot").Value!;
var recorderVersion = assembly.GetName().Version!.ToString(3);
var repository = FindRepository(Directory.GetCurrentDirectory()) ?? FindRepository(AppContext.BaseDirectory)
    ?? throw new InvalidOperationException("Run the recorder from inside the desktop host's repository: no folder holding Cargo.toml and parity/ was found.");
var parity = Path.Combine(repository, "parity");
var scriptsRoot = Path.Combine(parity, "scripts");

// The oracle is the standalone host at the reference commit, unchanged.
var commit = Git(standaloneRoot, "rev-parse HEAD").Trim();
if (commit != referenceCommit && !options.Contains("--any-commit"))
{
    Console.Error.WriteLine($"The standalone host at {standaloneRoot} is at {commit}, not the reference commit {referenceCommit}. Pass --any-commit to record from it anyway.");
    return 2;
}

var dirty = Git(standaloneRoot, "status --porcelain --untracked-files=no").Trim();
if (dirty.Length > 0 && !options.Contains("--allow-dirty"))
{
    Console.Error.WriteLine($"The standalone host at {standaloneRoot} has local changes, so it is not the reference commit's code:\n{dirty}\nPass --allow-dirty to record from it anyway.");
    return 2;
}

// What the recorder says goes to the console it started with; the standalone host's own logging,
// which also writes to the console, is silenced unless asked for.
var output = Console.Out;
if (!options.Contains("--verbose"))
{
    Console.SetOut(TextWriter.Null);
    // The host reads its Serilog levels from configuration, environment variables included.
    Environment.SetEnvironmentVariable("Serilog__MinimumLevel__Default", "Warning");
    Environment.SetEnvironmentVariable("Serilog__MinimumLevel__Override__Microsoft.Hosting.Lifetime", "Warning");
}

IEnumerable<string> roots = targets.Count == 0 ? [scriptsRoot] : targets.Select(Path.GetFullPath);
var scripts = roots
    .SelectMany(target => Directory.Exists(target)
        ? Directory.EnumerateFiles(target, "*.json", SearchOption.AllDirectories).Order(StringComparer.Ordinal)
        : (IEnumerable<string>)[target])
    .ToList();
if (scripts.Count == 0)
{
    Console.Error.WriteLine($"No scripts found under {scriptsRoot}.");
    return 2;
}

// WebApplicationFactory finds the standalone host's content root through the manifest beside the
// recorder, which it looks for in the current directory; every path above is absolute by now.
Environment.CurrentDirectory = AppContext.BaseDirectory;

var failures = 0;
// Ctrl+C stops the replay in hand; the source lives as long as the process, so it is not disposed.
var cancellation = new CancellationTokenSource();
Console.CancelKeyPress += (_, e) =>
{
    e.Cancel = true;
    cancellation.Cancel();
};

foreach (var scriptPath in scripts)
{
    try
    {
        var script = EditScript.Load(scriptPath, scriptsRoot);
        output.WriteLine($"Recording {script.Tool}/{script.Name} ({script.EditCount} edits in {script.Steps.Count} steps)");
        await RecordAsync(script, cancellation.Token);
    }
    catch (Exception exception) when (exception is ScriptException or InvalidOperationException or IOException or JsonException or Grpc.Core.RpcException)
    {
        failures++;
        Console.Error.WriteLine($"FAILED {Path.GetRelativePath(repository, scriptPath)}: {exception.Message}");
    }
}

output.WriteLine(failures == 0 ? $"Recorded {scripts.Count} script(s)." : $"{failures} of {scripts.Count} script(s) failed.");
return failures == 0 ? 0 : 1;

async Task RecordAsync(EditScript script, CancellationToken cancellationToken)
{
    if (!File.Exists(Path.Combine(parity, "fixtures", script.Document)))
    {
        throw new ScriptException(script.SourcePath, $"opens {script.Document}, which is not under parity/fixtures/");
    }

    // A write the standalone host could not make (see Replay.Settle) fails the replay, never the
    // golden: the whole replay runs again on a fresh host, so what is recorded is one clean run.
    for (var attempt = 1; ; attempt++)
    {
        try
        {
            await RecordOnceAsync(script, cancellationToken);
            return;
        }
        catch (ScriptException exception) when (attempt < 3 && exception.Message.Contains("could not be written", StringComparison.Ordinal))
        {
            output.WriteLine($"  attempt {attempt} hit a failed write of the standalone host, recording again: {exception.Message}");
        }
    }
}

async Task RecordOnceAsync(EditScript script, CancellationToken cancellationToken)
{
    var fixtureFolder = Path.Combine(parity, "fixtures", script.FixtureFolder);

    // A host of its own per script, in the developer environment, its project list and problem
    // cache in a temporary folder: no script sees another's history, and nothing reaches the
    // person's own application data.
    using var host = new HostFixture();
    var projectFolder = host.NewProjectFolder(new DirectoryInfo(fixtureFolder).Name);
    CopyFolder(fixtureFolder, projectFolder);
    var before = Snapshot(projectFolder);
    var initialText = string.Join("\n", before.Values.Select(bytes => System.Text.Encoding.UTF8.GetString(bytes)));

    var result = await Replay.RunAsync(host, script, projectFolder, initialText, options.Contains("--elements") ? output : null, cancellationToken);
    if (result.Notices.Count > 0)
    {
        // A notice is the standalone host saying a command succeeded but something beside it - a
        // layout sidecar, say - could not be written: the files are not what the script made.
        throw new ScriptException(script.SourcePath, $"a file beside the document could not be written: {string.Join("; ", result.Notices)}");
    }

    var after = Snapshot(projectFolder);
    var changed = after.Where(file => !before.TryGetValue(file.Key, out var old) || !old.AsSpan().SequenceEqual(file.Value))
        .Select(file => file.Key).Order(StringComparer.Ordinal).ToList();
    var removed = before.Keys.Where(file => !after.ContainsKey(file)).Order(StringComparer.Ordinal).ToList();

    var goldens = Path.Combine(parity, "goldens", script.Tool.Replace('/', Path.DirectorySeparatorChar), script.Name.Replace('/', Path.DirectorySeparatorChar));
    if (Directory.Exists(goldens))
    {
        Directory.Delete(goldens, recursive: true);
    }

    Directory.CreateDirectory(goldens);
    foreach (var file in changed)
    {
        var target = Path.Combine(goldens, file.Replace('/', Path.DirectorySeparatorChar));
        Directory.CreateDirectory(Path.GetDirectoryName(target)!);
        await File.WriteAllBytesAsync(target, after[file], cancellationToken);
    }

    var record = new
    {
        script = Path.GetRelativePath(repository, script.SourcePath).Replace('\\', '/'),
        tool = script.Tool,
        fixture = script.Fixture,
        registration = script.Registration,
        standaloneCommit = commit,
        recorderVersion,
        // The standalone host writes some files with the platform's line ending (System.Text.Json's
        // indented output, for one), so a golden is the standalone host's bytes on this platform.
        platform = OperatingSystem.IsWindows() ? "windows" : OperatingSystem.IsMacOS() ? "macos" : "linux",
        recordedAt = DateTimeOffset.UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'", System.Globalization.CultureInfo.InvariantCulture),
        edits = script.EditCount,
        changed,
        removed,
        generated = result.Generated.Select(g => new { step = g.Step, id = g.Id, type = g.Type, connection = g.Connection }),
        steps = result.Steps.Select(s => new { step = s.Step, kind = s.Kind, rpc = s.Rpc, outcome = s.Outcome, detail = s.Detail }),
    };
    var json = JsonSerializer.Serialize(record, new JsonSerializerOptions { WriteIndented = true, Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping });
    await File.WriteAllTextAsync(Path.Combine(goldens, "recorded.json"), json.ReplaceLineEndings("\n") + "\n", cancellationToken);

    output.WriteLine($"  changed: {(changed.Count == 0 ? "nothing" : string.Join(", ", changed))}{(removed.Count > 0 ? $"; removed: {string.Join(", ", removed)}" : "")}; generated ids: {result.Generated.Count}");
}

static Dictionary<string, byte[]> Snapshot(string folder) =>
    Directory.EnumerateFiles(folder, "*", SearchOption.AllDirectories)
        .ToDictionary(file => Path.GetRelativePath(folder, file).Replace('\\', '/'), File.ReadAllBytes, StringComparer.Ordinal);

static void CopyFolder(string from, string to)
{
    foreach (var file in Directory.EnumerateFiles(from, "*", SearchOption.AllDirectories))
    {
        var target = Path.Combine(to, Path.GetRelativePath(from, file));
        Directory.CreateDirectory(Path.GetDirectoryName(target)!);
        File.Copy(file, target);
    }
}

static string? FindRepository(string start)
{
    for (var folder = new DirectoryInfo(start); folder is not null; folder = folder.Parent)
    {
        if (File.Exists(Path.Combine(folder.FullName, "Cargo.toml")) && Directory.Exists(Path.Combine(folder.FullName, "parity")))
        {
            return folder.FullName;
        }
    }

    return null;
}

static string Git(string folder, string arguments)
{
    var start = new ProcessStartInfo("git") { RedirectStandardOutput = true, UseShellExecute = false };
    start.ArgumentList.Add("-C");
    start.ArgumentList.Add(folder);
    foreach (var argument in arguments.Split(' '))
    {
        start.ArgumentList.Add(argument);
    }

    using var git = Process.Start(start) ?? throw new InvalidOperationException("git could not be started.");
    var output = git.StandardOutput.ReadToEnd();
    git.WaitForExit();
    return git.ExitCode == 0 ? output : throw new InvalidOperationException($"git {arguments} failed in {folder}.");
}

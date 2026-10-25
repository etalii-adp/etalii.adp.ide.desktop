#if STANDALONE_HIERARCHY_SURFACE
using EtAlii.Adp.Diagram.Wire;
using Grpc.Core;
using DesignerWire = EtAlii.Adp.Designer.Wire;
using Path = EtAlii.Adp.Documents.Wire.Path;
using WireShortGuid = EtAlii.Adp.Documents.Wire.ShortGuid;

namespace EtAlii.Adp.Parity.Recorder;

/// <summary>
/// The replay of a script of a designer of the <c>hierarchy</c> kind (etalii.adp spec 036, contracts/parity-scripts.md):
/// the document opened as a page on the tab's one stream (<c>WorkspaceService.Watch</c> and
/// <c>DesignerService.OpenHierarchySurface</c>, as the standalone host's client opens it), each <c>hierarchy</c> step
/// sent as a gesture with <c>DesignerService.HierarchySurfaceGesture</c> and waited for until its outcome says whether it
/// was written, each <c>outside</c> step written to the file outside the host and waited for until the page is read
/// again, and undo and redo on the project's history as for a diagram.
/// </summary>
internal sealed partial class Replay
{
    private static readonly TimeSpan OutsidePatience = TimeSpan.FromSeconds(10);

    private AsyncServerStreamingCall<WorkspaceMessage>? _hierarchyCall;
    private Task? _hierarchyReader;
    private DesignerWire.DesignerService.DesignerServiceClient? _surface;
    private readonly WireShortGuid _hierarchyStream = ShortGuid.NewShortGuid();
    private readonly Lock _hierarchyLock = new();
    private readonly List<DesignerWire.HierarchySurfaceEditOutcome> _hierarchyOutcomes = [];
    private int _outsideModels;

    /// <summary>Whether the script's tool type is a designer whose document is a hierarchy surface.</summary>
    private bool IsHierarchy => _script.Tool == "etalii/hierarchical-requirement";

    private async Task OpenHierarchyAsync(CancellationToken cancellationToken)
    {
        _surface = new DesignerWire.DesignerService.DesignerServiceClient(_channel);
        var workspace = new WorkspaceService.WorkspaceServiceClient(_channel);

        // The tab's one stream first: its first message is the sign the connection is registered.
        _hierarchyCall = workspace.Watch(new WatchWorkspaceRequest { ProjectId = _projectId, WatchId = _watchId }, _headers, cancellationToken: _streams.Token);
        using (var first = Timeout(cancellationToken, PromptTimeout))
        {
            if (!await _hierarchyCall.ResponseStream.MoveNext(first.Token))
            {
                throw new InvalidOperationException("The workspace stream ended before the page opened.");
            }
        }

        var opened = new TaskCompletionSource();
        _hierarchyReader = Task.Run(() => ReadHierarchyAsync(opened), CancellationToken.None);

        _entryId = await EntryIdAsync(_script.DocumentInProject, cancellationToken);
        _documentPath = Path.Of(_script.DocumentInProject);
        await _surface.OpenHierarchySurfaceAsync(
            new DesignerWire.OpenHierarchySurfaceRequest { ProjectId = _projectId, WatchId = _watchId, StreamId = _hierarchyStream, Path = _documentPath },
            _headers,
            cancellationToken: cancellationToken);
        using (var first = Timeout(cancellationToken, PromptTimeout))
        {
            await opened.Task.WaitAsync(first.Token);
        }

        await WaitUntilQuietAsync(cancellationToken);
    }

    private async Task ReadHierarchyAsync(TaskCompletionSource opened)
    {
        try
        {
            while (await _hierarchyCall!.ResponseStream.MoveNext(_streams.Token))
            {
                var message = _hierarchyCall.ResponseStream.Current;
                if (message.MessageCase == WorkspaceMessage.MessageOneofCase.Context && message.Context.MessageCase == Context.Wire.ContextMessage.MessageOneofCase.Notice)
                {
                    lock (_notices)
                    {
                        _notices.Add($"step {_currentStep}: {message.Context.Notice.Message}");
                    }
                }

                // A prompt is an action's, such as the explorer's add, as the diagram's stream hands it on.
                if (message.MessageCase == WorkspaceMessage.MessageOneofCase.Context && message.Context.MessageCase == Context.Wire.ContextMessage.MessageOneofCase.Prompt)
                {
                    await _prompts.Writer.WriteAsync(message.Context.Prompt, _streams.Token);
                }

                if (message.MessageCase != WorkspaceMessage.MessageOneofCase.HierarchySurface || !message.HierarchySurface.StreamId.Equals(_hierarchyStream))
                {
                    continue;
                }

                var surface = message.HierarchySurface;
                lock (_hierarchyLock)
                {
                    switch (surface.EventCase)
                    {
                        case DesignerWire.HierarchySurfaceStreamMessage.EventOneofCase.Model:
                            if (surface.Model.Cause == "outside")
                            {
                                _outsideModels++;
                            }

                            opened.TrySetResult();
                            break;
                        case DesignerWire.HierarchySurfaceStreamMessage.EventOneofCase.Outcome:
                            _hierarchyOutcomes.Add(surface.Outcome);
                            break;
                        case DesignerWire.HierarchySurfaceStreamMessage.EventOneofCase.Ended:
                            opened.TrySetException(new InvalidOperationException("The standalone host ended the page's stream."));
                            break;
                    }
                }

                Interlocked.Exchange(ref _lastDeltaTicks, DateTime.UtcNow.Ticks);
            }
        }
        catch (Exception exception) when (exception is OperationCanceledException or RpcException { StatusCode: StatusCode.Cancelled })
        {
            opened.TrySetCanceled();
        }
        catch (RpcException exception)
        {
            opened.TrySetException(new InvalidOperationException($"The standalone host refused to open {_script.DocumentInProject}: {exception.Status.Detail}", exception));
        }
    }

    private async Task<(string, string?, string)> RunHierarchyAsync(HierarchyStep step, CancellationToken cancellationToken)
    {
        WireShortGuid editId = ShortGuid.NewShortGuid();
        var request = new DesignerWire.HierarchySurfaceGestureRequest
        {
            WatchId = _watchId,
            StreamId = _hierarchyStream,
            EditId = editId,
            Operation = step.Operation,
            TargetId = step.Target,
        };
        foreach (var (name, value) in step.Params)
        {
            request.Params[name] = value;
        }

        var detail = $"{step.Operation} {step.Target}".TrimEnd();
        var answer = await _surface!.HierarchySurfaceGestureAsync(request, _headers, cancellationToken: cancellationToken);
        if (answer.Error.Length > 0)
        {
            return ("DesignerService.HierarchySurfaceGesture", answer.Error, detail);
        }

        // Accepted at once and written behind: the step is done when its outcome says so.
        var started = DateTime.UtcNow;
        while (DateTime.UtcNow - started < OutcomeTimeout)
        {
            DesignerWire.HierarchySurfaceEditOutcome? outcome;
            lock (_hierarchyLock)
            {
                outcome = _hierarchyOutcomes.FirstOrDefault(candidate => candidate.EditId.Equals(editId));
            }

            if (outcome is not null)
            {
                return ("DesignerService.HierarchySurfaceGesture", outcome.Written ? null : $"could not be written: {outcome.Error}", detail);
            }

            await Task.Delay(50, cancellationToken);
        }

        return ("DesignerService.HierarchySurfaceGesture", "no outcome arrived for the gesture", detail);
    }

    /// <summary>Writes the file as another program would, and waits for the page to be read again from outside.</summary>
    private async Task<(string, string?, string)> RunOutsideAsync(OutsideStep step, CancellationToken cancellationToken)
    {
        var path = System.IO.Path.Combine(_projectFolder, step.File);
        var text = await File.ReadAllTextAsync(path, cancellationToken);
        var at = text.IndexOf(step.From, StringComparison.Ordinal);
        if (at < 0)
        {
            return ("File.WriteAllText", $"'{step.From}' is not in {step.File}", step.File);
        }

        int before;
        lock (_hierarchyLock)
        {
            before = _outsideModels;
        }

        await File.WriteAllTextAsync(path, string.Concat(text.AsSpan(0, at), step.To, text.AsSpan(at + step.From.Length)), cancellationToken);
        var started = DateTime.UtcNow;
        while (DateTime.UtcNow - started < OutsidePatience)
        {
            lock (_hierarchyLock)
            {
                if (_outsideModels > before)
                {
                    return ("File.WriteAllText", null, $"{step.File}: '{step.From}' to '{step.To}'");
                }
            }

            await Task.Delay(50, cancellationToken);
        }

        return ("File.WriteAllText", "the page was not read again after the file changed", step.File);
    }

    private async Task DisposeHierarchyAsync()
    {
        if (_hierarchyReader is not null)
        {
            try
            {
                await _hierarchyReader;
            }
            catch (Exception exception) when (exception is OperationCanceledException or RpcException)
            {
                // Ended with the replay.
            }
        }

        _hierarchyCall?.Dispose();
    }
}
#else
namespace EtAlii.Adp.Parity.Recorder;

/// <summary>
/// A worktree without the hierarchy surface, as 8b6ccfc is: no script of a designer of the <c>hierarchy</c> kind is
/// recorded from it, since such a script's reference commit is another (README.md, "A second reference commit").
/// </summary>
internal sealed partial class Replay
{
    private bool IsHierarchy => false;

    private Task OpenHierarchyAsync(CancellationToken cancellationToken) => throw new InvalidOperationException("This standalone host has no hierarchy surface.");

    private Task<(string, string?, string)> RunHierarchyAsync(HierarchyStep step, CancellationToken cancellationToken) => throw new InvalidOperationException("This standalone host has no hierarchy surface.");

    private Task<(string, string?, string)> RunOutsideAsync(OutsideStep step, CancellationToken cancellationToken) => throw new InvalidOperationException("This standalone host has no hierarchy surface.");

    private Task DisposeHierarchyAsync() => Task.CompletedTask;
}
#endif

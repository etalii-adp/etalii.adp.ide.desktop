using System.Text.RegularExpressions;
using EtAlii.Adp.Diagram.Wire;
using Grpc.Core;
using TableWire = EtAlii.Adp.Designer.Wire;
using Path = EtAlii.Adp.Documents.Wire.Path;
using WireShortGuid = EtAlii.Adp.Documents.Wire.ShortGuid;

namespace EtAlii.Adp.Parity.Recorder;

/// <summary>
/// The replay of a designer's script: the document opened as a table on the tab's one stream
/// (<c>WorkspaceService.Watch</c> and <c>DesignerService.OpenTable</c>, as the standalone host's
/// <c>useTableStream</c> opens it), each <c>table</c> step sent as a gesture with
/// <c>DesignerService.Edit</c> and waited for until its outcome says it was written, each
/// <c>view</c> step sent with <c>DesignerService.SetTableView</c>, and undo and redo on the
/// project's history as for a diagram. An id the standalone host generated is found in the
/// project's files after the step that wrote it.
/// </summary>
internal sealed partial class Replay
{
    private static readonly TimeSpan OutcomeTimeout = TimeSpan.FromSeconds(20);
    private static readonly TimeSpan ViewPatience = TimeSpan.FromSeconds(3);

    private AsyncServerStreamingCall<WorkspaceMessage>? _workspaceCall;
    private Task? _workspaceReader;
    private TableWire.DesignerService.DesignerServiceClient? _designer;
    private readonly WireShortGuid _tableStream = ShortGuid.NewShortGuid();
    private readonly Lock _tableLock = new();
    private readonly List<TableWire.TableEditOutcome> _outcomes = [];
    private string _viewId = "";

    /// <summary>Whether the script's tool type is a designer whose document is a table.</summary>
    private bool IsTable => _script.Tool == "etalii/knowledge";

    [GeneratedRegex("(?<![0-9a-z])[0-9a-z]{25}(?![0-9a-z])")]
    private static partial Regex ShortGuidText();

    private async Task OpenTableAsync(CancellationToken cancellationToken)
    {
        _designer = new TableWire.DesignerService.DesignerServiceClient(_channel);
        var workspace = new WorkspaceService.WorkspaceServiceClient(_channel);

        // The tab's one stream first: its first message is the sign the connection is registered.
        _workspaceCall = workspace.Watch(new WatchWorkspaceRequest { ProjectId = _projectId, WatchId = _watchId }, _headers, cancellationToken: _streams.Token);
        using (var first = Timeout(cancellationToken, PromptTimeout))
        {
            if (!await _workspaceCall.ResponseStream.MoveNext(first.Token))
            {
                throw new InvalidOperationException("The workspace stream ended before its baseline.");
            }
        }

        var baseline = new TaskCompletionSource();
        _workspaceReader = Task.Run(() => ReadWorkspaceAsync(baseline), CancellationToken.None);

        _entryId = await EntryIdAsync(_script.DocumentInProject, cancellationToken);
        _documentPath = Path.Of(_script.DocumentInProject);
        await _designer.OpenTableAsync(
            new TableWire.OpenTableRequest { ProjectId = _projectId, WatchId = _watchId, StreamId = _tableStream, Path = _documentPath },
            _headers,
            cancellationToken: cancellationToken);
        using (var first = Timeout(cancellationToken, PromptTimeout))
        {
            await baseline.Task.WaitAsync(first.Token);
        }

        // The rows in sight, as a tab asks for them once it is laid out.
        await _designer.SetTableWindowAsync(
            new TableWire.SetTableWindowRequest { WatchId = _watchId, StreamId = _tableStream, First = 0, Count = 60 },
            _headers,
            cancellationToken: cancellationToken);
        await WaitUntilQuietAsync(cancellationToken);
    }

    private async Task ReadWorkspaceAsync(TaskCompletionSource baseline)
    {
        try
        {
            while (await _workspaceCall!.ResponseStream.MoveNext(_streams.Token))
            {
                var message = _workspaceCall.ResponseStream.Current;
                if (message.MessageCase == WorkspaceMessage.MessageOneofCase.Context && message.Context.MessageCase == Context.Wire.ContextMessage.MessageOneofCase.Notice)
                {
                    lock (_notices)
                    {
                        _notices.Add($"step {_currentStep}: {message.Context.Notice.Message}");
                    }
                }

                if (message.MessageCase != WorkspaceMessage.MessageOneofCase.Table || !message.Table.StreamId.Equals(_tableStream))
                {
                    continue;
                }

                var table = message.Table;
                lock (_tableLock)
                {
                    switch (table.EventCase)
                    {
                        case TableWire.TableStreamMessage.EventOneofCase.Baseline:
                            _viewId = table.Baseline.Settings?.ViewId ?? "";
                            baseline.TrySetResult();
                            break;
                        case TableWire.TableStreamMessage.EventOneofCase.Change when table.Change.ChangeCase == TableWire.TableChange.ChangeOneofCase.Structure:
                            _viewId = table.Change.Structure.Settings?.ViewId ?? _viewId;
                            break;
                        case TableWire.TableStreamMessage.EventOneofCase.Change when table.Change.ChangeCase == TableWire.TableChange.ChangeOneofCase.Outcome:
                            _outcomes.Add(table.Change.Outcome);
                            break;
                        case TableWire.TableStreamMessage.EventOneofCase.Ended:
                            baseline.TrySetException(new InvalidOperationException("The standalone host ended the table stream."));
                            break;
                    }
                }

                Interlocked.Exchange(ref _lastDeltaTicks, DateTime.UtcNow.Ticks);
            }
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

    private async Task<(string, string?, string)> RunTableAsync(TableStep step, CancellationToken cancellationToken)
    {
        string view;
        lock (_tableLock)
        {
            view = _viewId;
        }

        // Every gesture says which view it was made in, as the Knowledge designer's panel sends it.
        var gesture = new TableWire.TableGesture
        {
            Kind = step.Kind,
            RowId = Resolve(step.Row ?? ""),
            ColumnId = Resolve(step.Column ?? ""),
            ViewId = step.View is null ? view : Resolve(step.View),
            Index = step.At,
            TargetId = Resolve(step.Target ?? ""),
        };
        gesture.Values.AddRange(step.Values.Select(Resolve));
        foreach (var (name, value) in step.Settings)
        {
            gesture.Settings[name] = Resolve(value);
        }

        var detail = $"{step.Kind} {gesture}";
        WireShortGuid editId = ShortGuid.NewShortGuid();
        var answer = await _designer!.EditAsync(
            new TableWire.TableEditRequest { WatchId = _watchId, StreamId = _tableStream, EditId = editId, Gesture = gesture },
            _headers,
            cancellationToken: cancellationToken);
        if (answer.Error.Length > 0)
        {
            return ("DesignerService.Edit", answer.Error, detail);
        }

        // Accepted at once and written behind: the step is done when its outcome says so.
        var started = DateTime.UtcNow;
        while (DateTime.UtcNow - started < OutcomeTimeout)
        {
            TableWire.TableEditOutcome? outcome;
            lock (_tableLock)
            {
                outcome = _outcomes.FirstOrDefault(candidate => candidate.EditId.Equals(editId));
            }

            if (outcome is not null)
            {
                return ("DesignerService.Edit", outcome.Written ? null : $"could not be written: {outcome.Error}", detail);
            }

            await Task.Delay(50, cancellationToken);
        }

        return ("DesignerService.Edit", "no outcome arrived for the edit", detail);
    }

    private async Task<(string, string?, string)> RunViewAsync(ViewStep step, CancellationToken cancellationToken)
    {
        int before;
        lock (_tableLock)
        {
            before = _outcomes.Count;
        }

        var viewId = Resolve(step.ViewId);
        await _designer!.SetTableViewAsync(new TableWire.SetTableViewRequest { WatchId = _watchId, StreamId = _tableStream, ViewId = viewId }, _headers, cancellationToken: cancellationToken);

        // The view a file is left in is written as an edit, whose id is the session's own: the step waits for an outcome.
        var started = DateTime.UtcNow;
        while (DateTime.UtcNow - started < ViewPatience)
        {
            lock (_tableLock)
            {
                if (_outcomes.Count > before)
                {
                    var outcome = _outcomes[^1];
                    return ("DesignerService.SetTableView", outcome.Written ? null : $"could not be written: {outcome.Error}", viewId);
                }
            }

            await Task.Delay(50, cancellationToken);
        }

        return ("DesignerService.SetTableView", null, $"{viewId}, which wrote nothing");
    }

    /// <summary>
    /// The ids a step of a table wrote that were never in the project before: a ShortGuid is 25
    /// characters of base 36, and every one the fixture holds is in its initial text. Listed in the
    /// order they stand in the files.
    /// </summary>
    private void TrackGeneratedInFiles()
    {
        foreach (var file in Directory.EnumerateFiles(_projectFolder, "*", SearchOption.AllDirectories).Order(StringComparer.Ordinal))
        {
            var text = File.ReadAllText(file);
            foreach (Match match in ShortGuidText().Matches(text))
            {
                lock (_elementsLock)
                {
                    if (!_initialText.Contains(match.Value, StringComparison.Ordinal) && _seen.Add(match.Value))
                    {
                        _generated.Add(new GeneratedId(_currentStep, match.Value, "etalii/knowledge+id", Connection: false));
                    }
                }
            }
        }
    }

    private async Task DisposeTableAsync()
    {
        if (_workspaceReader is not null)
        {
            try
            {
                await _workspaceReader;
            }
            catch (Exception exception) when (exception is OperationCanceledException or RpcException)
            {
                // Ended with the replay.
            }
        }

        _workspaceCall?.Dispose();
    }
}

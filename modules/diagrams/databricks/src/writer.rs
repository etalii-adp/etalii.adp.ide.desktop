//! The named splice operations on a Databricks file: resources, tasks and libraries in and out, dependencies
//! connected and disconnected, keys renamed with every reference, scalars rewritten; only the lines an edit concerns
//! change (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/BundleWriter.cs`,
//! `JobWriter.cs`, `PipelineWriter.cs` and `DatabricksSplices.cs` at `8b6ccfc`. Every operation answers with the
//! sentence explaining why it refused, before any splice, never write-then-repair, so a refused document is untouched.
//! A successful splice invalidates the model that located it: callers read again. A pipeline is declared in block YAML
//! inside a bundle and in flow JSON as a settings file, and the JSON operations assume the layout such a file
//! conventionally has, one array item per line.

use crate::lines::{LineDocument, LineRange};
use crate::model::{BundleModel, JobDependency, JobModel, JobTask, PipelineModel};

/// What a writer answers: done, or why not.
pub type Written = Result<(), String>;

// ---- the shared splices (`DatabricksSplices`) ----------------------------------------------------------------------

/// Whether the document is written in JSON's flow syntax rather than block YAML: its first line that is not blank
/// starts with `{`.
pub fn is_json(document: &LineDocument) -> bool {
    document
        .lines()
        .iter()
        .find(|line| !line.is_blank())
        .is_some_and(|line| line.text.trim_start().starts_with('{'))
}

/// The first line inside `range` whose key is `key`, a dash prefix and either syntax's quoting tolerated.
pub fn find_key(document: &LineDocument, range: LineRange, key: &str) -> Option<usize> {
    let lines = document.lines();
    (range.start..=range.end).take_while(|i| *i < lines.len()).find(|i| {
        let trimmed = lines[*i].text.trim_start();
        let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        trimmed.starts_with(&format!("{key}:")) || trimmed.starts_with(&format!("\"{key}\":"))
    })
}

/// The leading white space of a line's text.
pub fn indent(text: &str) -> String {
    text[..text.len() - text.trim_start().len()].to_owned()
}

/// The indentation the keys inside a range already use: copied from the file rather than imposed.
pub fn key_indent_within(document: &LineDocument, range: LineRange) -> String {
    let lines = document.lines();
    for i in range.start + 1..=range.end {
        if i >= lines.len() {
            break;
        }
        if !lines[i].text.trim().is_empty() {
            return indent(&lines[i].text);
        }
    }
    // A one-line construct has no neighbour to copy, so its own indentation plus one step puts the new key under it.
    format!("{}  ", indent(&lines[range.start].text))
}

/// `value` with its backslashes and double quotes escaped, in double quotes.
fn double_quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A YAML value quoted only where the syntax needs it, so an ordinary value stays unquoted.
pub fn quote(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_owned();
    }
    let needs_quoting = value.contains(':')
        || value.contains('#')
        || value.starts_with('-')
        || value.starts_with(' ')
        || value.ends_with(' ')
        || value.starts_with('"')
        || value.starts_with('\'')
        || value.starts_with('{')
        || value.starts_with('[');
    if needs_quoting {
        double_quoted(value)
    } else {
        value.to_owned()
    }
}

/// The line `line` with everything after its first colon replaced by ` {value}`.
fn rewritten(document: &LineDocument, line: usize, value: &str) -> String {
    let text = document.line(line);
    let head = text.find(':').map_or("", |colon| &text[..=colon]);
    format!("{head} {value}")
}

fn replace_line(document: &mut LineDocument, line: usize, text: String) {
    document.replace(LineRange::new(line, line), &[text]);
}

// ---- the bundle (`BundleWriter`) -----------------------------------------------------------------------------------

/// The line declaring a root-level key, at column zero, so a target's own nested `resources:` is never mistaken for
/// the document's.
fn root_key_line(document: &LineDocument, key: &str) -> Option<usize> {
    let prefix = format!("{key}:");
    document.lines().iter().position(|line| line.text.starts_with(&prefix))
}

/// The last line still belonging to the root key starting at `key_line`: everything up to the next column-zero key.
fn region_end(document: &LineDocument, key_line: usize) -> usize {
    let lines = document.lines();
    for (i, line) in lines.iter().enumerate().skip(key_line + 1) {
        let first = line.text.chars().next();
        if first.is_some_and(|c| !c.is_whitespace() && c != '#') {
            return i - 1;
        }
    }
    lines.len() - 1
}

fn resource_skeleton(kind: &str, key: &str, key_indent: &str) -> Vec<String> {
    if kind == "jobs" {
        vec![
            format!("{key_indent}{key}:"),
            format!("{key_indent}  name: {key}"),
            format!("{key_indent}  tasks:"),
            format!("{key_indent}    - task_key: main"),
            format!("{key_indent}      notebook_task:"),
            format!("{key_indent}        notebook_path: notebooks/{key}"),
        ]
    } else {
        vec![
            format!("{key_indent}{key}:"),
            format!("{key_indent}  name: {key}"),
            format!("{key_indent}  libraries:"),
            format!("{key_indent}    - notebook:"),
            format!("{key_indent}        path: transformations/{key}"),
        ]
    }
}

/// Adds a new resource of `kind` under `resources:`, as a skeleton complete enough to open.
pub fn add_resource_skeleton(document: &mut LineDocument, bundle: &BundleModel, kind: &str, key: &str) -> Written {
    if key.is_empty() {
        return Err("A resource needs a key.".to_owned());
    }
    if kind != "jobs" && kind != "pipelines" {
        return Err(format!("There is no '{kind}' resource kind to add."));
    }
    if bundle.resources.iter().any(|r| r.kind == kind && r.key == key) {
        return Err(format!("A {kind} resource named '{key}' is already there."));
    }
    if let Some(last) = bundle.resources.iter().rfind(|r| r.kind == kind) {
        let key_indent = indent(document.line(last.lines.start));
        document.insert(last.lines.end + 1, &resource_skeleton(kind, key, &key_indent));
        return Ok(());
    }
    if let Some(resources_line) = root_key_line(document, "resources") {
        let last = document.lines().len() - 1;
        let kind_indent = key_indent_within(document, LineRange::new(resources_line, last));
        let mut lines = vec![format!("{kind_indent}{kind}:")];
        lines.extend(resource_skeleton(kind, key, &format!("{kind_indent}  ")));
        document.insert(resources_line + 1, &lines);
        return Ok(());
    }
    // No resources at all: the section is created at the end of the document with its first kind and entry.
    let mut lines = vec!["resources:".to_owned(), format!("  {kind}:")];
    lines.extend(resource_skeleton(kind, key, "    "));
    let end = document.lines().len();
    document.insert(end, &lines);
    Ok(())
}

/// Rewrites the bundle's name.
pub fn set_bundle_name(document: &mut LineDocument, name: &str) -> Written {
    if name.is_empty() {
        return Err("A bundle needs a name.".to_owned());
    }
    let Some(bundle_line) = root_key_line(document, "bundle") else {
        return Err("The file has no bundle: section to name.".to_owned());
    };
    let last = document.lines().len() - 1;
    let region = LineRange::new(bundle_line, last);
    let end = region_end(document, bundle_line);
    if let Some(line) = find_key(document, LineRange::new(bundle_line + 1, end), "name") {
        let text = rewritten(document, line, &quote(name));
        replace_line(document, line, text);
        return Ok(());
    }
    let key_indent = key_indent_within(document, region);
    document.insert(bundle_line + 1, &[format!("{key_indent}name: {}", quote(name))]);
    Ok(())
}

// ---- a job (`JobWriter`) -------------------------------------------------------------------------------------------

/// The skeleton each insertable task type opens with: its `*_task` key and starter fields, `{0}` the source.
fn task_skeleton(task_type: &str) -> Option<&'static [&'static str]> {
    Some(match task_type {
        "notebook" => &["notebook_task:", "  notebook_path: {0}"],
        "python" => &["spark_python_task:", "  python_file: {0}"],
        "wheel" => &["python_wheel_task:", "  package_name: {0}", "  entry_point: main"],
        "sql" => &["sql_task:", "  warehouse_id: \"\"", "  query:", "    query_id: {0}"],
        "pipeline" => &["pipeline_task:", "  pipeline_id: {0}"],
        "run-job" => &["run_job_task:", "  job_id: {0}"],
        "condition" => &[
            "condition_task:",
            "  op: EQUAL_TO",
            "  left: \"{0}\"",
            "  right: \"true\"",
        ],
        _ => return None,
    })
}

/// Where a new task goes and how it is indented: after the last task, copying its style, or directly under a bare
/// `tasks:` key; none when there is nowhere to insert.
fn task_insertion(document: &LineDocument, job: &JobModel) -> Option<(String, String, usize)> {
    if let Some(last) = job.tasks.last() {
        let item_indent = indent(document.line(last.lines.start));
        return Some((item_indent, key_indent_within(document, last.lines), last.lines.end + 1));
    }
    let tasks_line = find_key(document, job.lines, "tasks")?;
    let key_indent = indent(document.line(tasks_line));
    Some((format!("{key_indent}  "), format!("{key_indent}    "), tasks_line + 1))
}

/// Inserts a new task after the job's last one.
pub fn insert_task(
    document: &mut LineDocument,
    job: &JobModel,
    task_key: &str,
    task_type: &str,
    source: &str,
) -> Written {
    if task_key.is_empty() {
        return Err("A task needs a key.".to_owned());
    }
    if job.tasks.iter().any(|t| t.key == task_key) {
        return Err(format!("A task named '{task_key}' is already there."));
    }
    let Some(skeleton) = task_skeleton(task_type) else {
        return Err(format!("There is no '{task_type}' task type to add."));
    };
    let Some((item_indent, key_indent, insert_at)) = task_insertion(document, job) else {
        return Err("The job has no tasks section to add into.".to_owned());
    };
    let quoted = quote(source);
    let mut lines = vec![format!("{item_indent}- task_key: {task_key}")];
    lines.extend(
        skeleton
            .iter()
            .map(|part| format!("{key_indent}{}", part.replace("{0}", &quoted))),
    );
    document.insert(insert_at, &lines);
    Ok(())
}

/// What removing some of a task's dependencies splices: each entry's lines, and the `depends_on:` key line itself
/// when none would remain.
fn dependency_removals(document: &LineDocument, task: &JobTask, removed: &[&JobDependency]) -> Vec<LineRange> {
    let mut ranges: Vec<LineRange> = removed.iter().map(|d| d.lines).collect();
    if !removed.is_empty()
        && removed.len() == task.depends_on.len()
        && let Some(key_line) = find_key(document, task.lines, "depends_on")
    {
        ranges.push(LineRange::new(key_line, key_line));
    }
    ranges
}

/// Removes `ranges` from the bottom of the document upwards, so no removal shifts another's lines.
fn remove_descending(document: &mut LineDocument, mut ranges: Vec<LineRange>) {
    // A stable sort, descending by start: equal starts keep the order they were found in, as `OrderByDescending`.
    ranges.sort_by_key(|range| std::cmp::Reverse(range.start));
    for range in ranges {
        document.remove(range);
    }
}

/// Removes a task and every other task's `depends_on` reference to it.
pub fn remove_task(document: &mut LineDocument, job: &JobModel, task_key: &str) -> Written {
    let Some(task) = job.tasks.iter().find(|t| t.key == task_key) else {
        return Err(format!("There is no task named '{task_key}' any more."));
    };
    let mut ranges = vec![task.lines];
    for other in job.tasks.iter().filter(|t| t.key != task_key) {
        let removed: Vec<&JobDependency> = other.depends_on.iter().filter(|d| d.task_key == task_key).collect();
        ranges.extend(dependency_removals(document, other, &removed));
    }
    remove_descending(document, ranges);
    Ok(())
}

/// The dependencies that would go with a task, so an action can say how many before it runs.
pub fn references_to<'a>(job: &'a JobModel, task_key: &str) -> Vec<&'a JobDependency> {
    job.tasks
        .iter()
        .filter(|t| t.key != task_key)
        .flat_map(|t| &t.depends_on)
        .filter(|d| d.task_key == task_key)
        .collect()
}

/// Adds a `depends_on` entry: `to_key` comes to depend on `from_key`.
pub fn connect(document: &mut LineDocument, job: &JobModel, from_key: &str, to_key: &str, outcome: &str) -> Written {
    if from_key == to_key {
        return Err("A task cannot depend on itself.".to_owned());
    }
    let to = job.tasks.iter().find(|t| t.key == to_key);
    let Some(to) = to.filter(|_| job.tasks.iter().any(|t| t.key == from_key)) else {
        return Err("One of those tasks is not there any more.".to_owned());
    };
    if to.depends_on.iter().any(|d| d.task_key == from_key) {
        return Err(format!("'{to_key}' already depends on '{from_key}'."));
    }
    let key_indent = key_indent_within(document, to.lines);
    let entry_indent = match to.depends_on.last() {
        Some(last) => indent(document.line(last.lines.start)),
        None => format!("{key_indent}  "),
    };
    let mut entry = vec![format!("{entry_indent}- task_key: {from_key}")];
    if !outcome.is_empty() {
        entry.push(format!("{entry_indent}  outcome: \"{outcome}\""));
    }
    if let Some(last) = to.depends_on.last() {
        document.insert(last.lines.end + 1, &entry);
        return Ok(());
    }
    if let Some(depends_on_line) = find_key(document, to.lines, "depends_on") {
        document.insert(depends_on_line + 1, &entry);
        return Ok(());
    }
    // No depends_on yet: the key and its first entry go directly under the task_key line.
    let mut lines = vec![format!("{key_indent}depends_on:")];
    lines.extend(entry);
    document.insert(to.lines.start + 1, &lines);
    Ok(())
}

/// Removes one `depends_on` entry, and the `depends_on:` key itself when that was the last one.
pub fn disconnect(document: &mut LineDocument, job: &JobModel, from_key: &str, to_key: &str) -> Written {
    let to = job.tasks.iter().find(|t| t.key == to_key);
    let dependency = to.and_then(|t| t.depends_on.iter().find(|d| d.task_key == from_key));
    let (Some(to), Some(dependency)) = (to, dependency) else {
        return Err(format!(
            "There is no dependency from '{from_key}' to '{to_key}' any more."
        ));
    };
    let ranges = dependency_removals(document, to, &[dependency]);
    remove_descending(document, ranges);
    Ok(())
}

/// Renames a task, rewriting its own `task_key` and every `depends_on` reference to it in the same operation.
pub fn rename_task(document: &mut LineDocument, job: &JobModel, task_key: &str, new_key: &str) -> Written {
    if new_key.is_empty() {
        return Err("A task needs a key.".to_owned());
    }
    let Some(task) = job.tasks.iter().find(|t| t.key == task_key) else {
        return Err(format!("There is no task named '{task_key}' any more."));
    };
    if new_key != task_key && job.tasks.iter().any(|t| t.key == new_key) {
        return Err(format!("A task named '{new_key}' is already there."));
    }
    let Some(own) = find_key(document, task.lines, "task_key") else {
        return Err(format!("The task '{task_key}' has no task_key line to rewrite."));
    };
    let mut rewrites = vec![own];
    rewrites.extend(
        job.tasks
            .iter()
            .flat_map(|t| &t.depends_on)
            .filter(|d| d.task_key == task_key)
            .filter_map(|d| find_key(document, d.lines, "task_key")),
    );
    rewrites.sort_by_key(|line| std::cmp::Reverse(*line));
    for line in rewrites {
        let text = rewritten(document, line, new_key);
        replace_line(document, line, text);
    }
    Ok(())
}

fn set_task_key(document: &mut LineDocument, job: &JobModel, task_key: &str, key: &str, value: &str) -> Written {
    let Some(task) = job.tasks.iter().find(|t| t.key == task_key) else {
        return Err(format!("There is no task named '{task_key}' any more."));
    };
    let line = find_key(document, task.lines, key);
    if value.is_empty() {
        if let Some(line) = line {
            document.remove(LineRange::new(line, line));
        }
        return Ok(());
    }
    if let Some(line) = line {
        let text = rewritten(document, line, value);
        replace_line(document, line, text);
        return Ok(());
    }
    let key_indent = key_indent_within(document, task.lines);
    document.insert(task.lines.start + 1, &[format!("{key_indent}{key}: {value}")]);
    Ok(())
}

/// Rewrites a task's `run_if`; an empty value removes the key, restoring the default.
pub fn set_run_if(document: &mut LineDocument, job: &JobModel, task_key: &str, run_if: &str) -> Written {
    set_task_key(document, job, task_key, "run_if", run_if)
}

/// Rewrites a task's cluster binding; an empty value removes the key, which is serverless.
pub fn set_cluster(document: &mut LineDocument, job: &JobModel, task_key: &str, cluster_key: &str) -> Written {
    if !cluster_key.is_empty() && job.clusters.iter().all(|c| c.key != cluster_key) {
        return Err(format!("There is no cluster named '{cluster_key}' in this job."));
    }
    set_task_key(document, job, task_key, "job_cluster_key", cluster_key)
}

// ---- a pipeline (`PipelineWriter`) ---------------------------------------------------------------------------------

/// Appends a library entry after the pipeline's last one.
pub fn insert_library(document: &mut LineDocument, pipeline: &PipelineModel, kind: &str, path: &str) -> Written {
    if !matches!(kind, "notebook" | "file" | "glob") {
        return Err(format!("There is no '{kind}' library kind to add."));
    }
    if path.is_empty() {
        return Err("A library needs a path.".to_owned());
    }
    if pipeline.libraries.iter().any(|l| l.kind == kind && l.path == path) {
        return Err(format!("The {kind} library '{path}' is already there."));
    }
    let path_key = if kind == "glob" { "include" } else { "path" };
    if is_json(document) {
        let Some(last) = pipeline.libraries.last() else {
            return Err("The settings file has no libraries array to add into.".to_owned());
        };
        // The previous last item gains the comma that now separates it from the new one.
        let last = last.lines;
        let last_text = document.line(last.end).trim_end().to_owned();
        let item_indent = indent(document.line(last.start));
        replace_line(document, last.end, format!("{last_text},"));
        document.insert(
            last.end + 1,
            &[format!(
                "{item_indent}{{ \"{kind}\": {{ \"{path_key}\": \"{path}\" }} }}"
            )],
        );
        return Ok(());
    }
    let entry = |entry_indent: &str| {
        vec![
            format!("{entry_indent}- {kind}:"),
            format!("{entry_indent}    {path_key}: {}", quote(path)),
        ]
    };
    if let Some(last) = pipeline.libraries.last() {
        let lines = entry(&indent(document.line(last.lines.start)));
        document.insert(last.lines.end + 1, &lines);
        return Ok(());
    }
    if let Some(libraries_line) = find_key(document, pipeline.lines, "libraries") {
        let lines = entry(&format!("{}  ", indent(document.line(libraries_line))));
        document.insert(libraries_line + 1, &lines);
        return Ok(());
    }
    let key_indent = key_indent_within(document, pipeline.lines);
    let mut lines = vec![format!("{key_indent}libraries:")];
    lines.extend(entry(&format!("{key_indent}  ")));
    document.insert(pipeline.lines.end + 1, &lines);
    Ok(())
}

/// Removes one library entry. The last one is refused rather than removed: a pipeline with no libraries has nothing
/// to run.
pub fn remove_library(document: &mut LineDocument, pipeline: &PipelineModel, kind: &str, path: &str) -> Written {
    let Some(index) = pipeline.libraries.iter().position(|l| l.kind == kind && l.path == path) else {
        return Err(format!("There is no {kind} library '{path}' any more."));
    };
    if pipeline.libraries.len() == 1 {
        return Err("A pipeline needs at least one library; removing the last one is not allowed.".to_owned());
    }
    let library = &pipeline.libraries[index];
    let was_last = *library == pipeline.libraries[pipeline.libraries.len() - 1];
    document.remove(library.lines);
    if was_last && is_json(document) {
        // The item before it is the new last, and JSON forbids its trailing comma.
        let previous = pipeline.libraries[pipeline.libraries.len() - 2].lines;
        let text = document.line(previous.end).trim_end().to_owned();
        if let Some(stripped) = text.strip_suffix(',') {
            replace_line(document, previous.end, stripped.to_owned());
        }
    }
    Ok(())
}

/// Rewrites one of the pipeline's own scalars (name, catalog, schema, channel), keeping the file's syntax: quoting and
/// the trailing comma in JSON, the value style in YAML.
pub fn set_scalar(document: &mut LineDocument, pipeline: &PipelineModel, key: &str, value: &str) -> Written {
    if key == "name" && value.is_empty() {
        return Err("A pipeline needs a name.".to_owned());
    }
    let Some(line) = find_key(document, pipeline.lines, key) else {
        let key_indent = key_indent_within(document, pipeline.lines);
        if is_json(document) {
            return Err(format!("The settings file has no \"{key}\" to rewrite."));
        }
        document.insert(
            pipeline.lines.start + 1,
            &[format!("{key_indent}{key}: {}", quote(value))],
        );
        return Ok(());
    };
    if is_json(document) {
        let had_comma = document.line(line).trim_end().ends_with(',');
        let text = rewritten(document, line, &double_quoted(value));
        replace_line(document, line, format!("{text}{}", if had_comma { "," } else { "" }));
        return Ok(());
    }
    let text = rewritten(document, line, &quote(value));
    replace_line(document, line, text);
    Ok(())
}

# Updating the activity file

Give this text to an agent that works in a project with an activity file (`*.aad`). The file is the team's shared picture of who works on what and where; ADP draws it and redraws it the moment the file changes. An agent keeps its own entries true by editing the file as it works.

## The rules

1. **Change your own entries and leave everything else as it is**: other agents' entries, comments, keys you do not know, the order of things, and the line endings the file has.
2. **Never touch `view`.** That part belongs to the person looking at the diagram: where they locked an element, which groups they folded, whether archived specifications show.
3. **Give every new entry an `id`** that is unique in the file and that you will recognise again, such as `t-layout` or `pr-159`. Find your entries by their ids afterwards.
4. **Write `updated` whenever you add or change a task or a pull request**: the moment in ISO 8601 with the offset, to the second, such as `2026-10-09T16:55:00+02:00`. Tasks and pull requests are listed most recently updated first.
5. **Write the file in one go**, the whole text at once, and read it again just before you do: somebody else may have written it since you last looked.
6. **A status is one of the words below and nothing else.** A link is a web address or a path in the project.

## When to update, and how

**You start work on a specification.** Add yourself under `agents` if you are not there, and set `specification` to the specification's id. An agent works on one specification at a time, so this replaces what the key held. If the specification is `pending`, set its `status` to `progressing`.

```yaml
agents:
  - id: a-developer-2
    name: Developer 2
    specification: s-activity
```

**You work somewhere.** Add a location under `locations` for each branch and folder you work in, with `agent` set to your id and `environment` to the id of the system it is on. Leave `folder` out when you work in the main checkout; the diagram shows `Default`.

```yaml
locations:
  - id: l-activity
    agent: a-developer-2
    environment: e-fractal
    folder: .claude/worktrees/activity
    branch: features/activity
```

**A task's state changes.** Set the task's `status` and its `updated`. The statuses are `pending`, `progressing`, `input-required` and `finished`. Use `input-required` when you cannot go on without a person, and say what you need in the task's `title` or behind its `link`.

```yaml
      - id: t-layout
        title: The radiating layout
        status: finished
        updated: 2026-10-09T15:40:00+02:00
```

**You find or split off a task.** Add it to the specification's `tasks` with an `id`, a `title`, a `status` and `updated`.

**A specification's state changes.** Set its `status`: `pending`, `progressing`, `input-required`, `finished` or `archived`. Set `input-required` when the whole specification waits on a person, and `finished` when its last task is.

**You open or update a pull request.** Add it to your location's `pullRequests`, or change the one that is there, with `updated` and a `link` to it.

```yaml
    pullRequests:
      - id: pr-159
        title: "159: Open, draw and edit an activity file"
        updated: 2026-10-09T18:20:00+02:00
        link: https://github.com/etalii-adp/etalii.adp.ide.standalone/pull/159
```

**You finish.** Set your tasks and, when it is done, the specification to `finished`. Remove your `specification` key, so that the diagram shows you as free, and remove the locations you no longer work in. Leave the specification, its tasks and the pull requests' history where they are: archiving is a person's decision.

## The whole file

The file's keys, their meaning and a complete example are in the definition's companion, `agent-activity-diagram.md`, and its JSON Schema is published beside it in etalii.adp. The examples in this folder are complete files to copy from.

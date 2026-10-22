# Ansible structure test fixtures

Four Ansible project trees the module's reader, graph, layout, rules and property provider are
tested against, and which the parity transcript (`../Parity/ansible-structure.transcript.json`) reads. This module is **read-only**: no test here may write into these trees, and
`ZeroWrites.Tests` (task 24) snapshots them byte-for-byte — contents and mtimes — to prove it.

`.gitattributes` marks everything here `-text`, so these bytes survive checkout on every
platform. With `core.autocrlf` on, git would otherwise rewrite the line endings and the
byte-identical assertion would be measuring git rather than the module.

**No fixture carries an `.adp`.** Requirement 2.3 is explicit that ADP never infers that a
folder is an Ansible project, so a test that wants one registered writes the `.adp` into a temp
copy. A fixture with a registration committed into it would quietly make the opposite claim.

## Provenance

**Every file here was written by hand for this repository.** None is a copy of an upstream
project, and none may be described as canonical.

Task 1 asked for the YAML to be validated against Ansible's own expectations where possible.
**That was not possible here and the fixtures are not machine-validated:** no Python, `ansible`
or `ansible-lint` is available in this environment (`python` resolves to the Windows Store
shim). The files were written against the [recommended directory
layout](https://charlesreid1.com/wiki/Ansible/Directory_Layout/Details) and Ansible's module
and keyword documentation, and they use fully-qualified module names (`ansible.builtin.*`)
throughout, but they carry no external certification.

This is a weaker guarantee than the C4 corpus has, and it is worth stating plainly rather than
implying otherwise: a disagreement between one of these files and real Ansible is **this
fixture's** bug, not necessarily the module's. If `ansible-lint` becomes available, running it
over `infrastructure/` and `unconventional/` — both of which are meant to be entirely valid —
is the cheapest way to find out.

One machine check does arrive, one task later and for free: from task 9 onwards every file here
is parsed by YamlDotNet as part of the reader's own tests, so a **malformed** fixture fails the
suite immediately. That covers YAML syntax and nothing above it — a file can parse perfectly
and still be Ansible no-one would write. `unparsable.yml` is the deliberate exception, and it
is the only file here that must fail to parse.

## `infrastructure/` — the best-practice tree

The worked example from the requirements, as a real tree. This is the fixture that must render
completely, validate **clean**, and describe every node kind.

| File | What it guards |
|---|---|
| `ansible.cfg` | An annotation on the project node, not a box (Requirement 4.1). |
| `site.yml` | The entry playbook: nothing imports it, so it is the layout's first rank. Two `import_playbook` edges (Requirement 5.2). |
| `webservers.yml` | One play, `hosts: web`, a plain `roles:` list — the simplest `UsesRole` case (Requirement 5.1). |
| `dbservers.yml` | A `roles:` entry in **mapping** form with a `when:`, so the reader must handle both the string and the mapping shape, and the condition must survive verbatim (Requirement 5.7). |
| `inventories/production/hosts.yml` | Defines `web` and `db`, so both plays' `hosts:` patterns resolve (Requirement 5.6). |
| `inventories/production/group_vars/{all,web}.yml` | A variable folder as a node (Requirement 4.1), with more than one file in it. |
| `inventories/production/host_vars/web-01…yml` | `host_vars` beside `group_vars` — the second variable-folder kind. |
| `inventories/staging/**` | A second environment, so "per environment" in Requirement 4.1 has two subjects and a play's `Targets` edge has two candidate inventories. |
| `collections/requirements.yml` | The second annotation kind. |
| `roles/common/**` | A role with `tasks` and `defaults` and **no** `meta` — the role that is depended upon but depends on nothing. Used by both plays, so it proves a shared role is drawn **once** (Requirement 6.3). |
| `roles/nginx/tasks/main.yml` | An `include_tasks` to a sibling task file — the dynamic, within-role edge (Requirement 5.3). |
| `roles/nginx/tasks/tls.yml` | The task file that edge points at, drawn because it is included (Requirement 4.1). |
| `roles/nginx/{handlers,templates}/**` | The canonical subfolders a role summary counts (Requirement 4.2). |
| `roles/nginx/meta/main.yml` | `dependencies:` — the role → role edge, in its own style (Requirement 5.5). Also carries `galaxy_info`, which the model does not read and must ignore. |
| `roles/postgres/**` | A second role depending on `common`, so the dependency fan-in is real rather than a single edge. |
| `Makefile` | Not Ansible. Ignored without complaint (Requirement 1.3). |
| `docs/runbook.md` | Not Ansible. Ignored. |
| `docs/topology.yml` | **YAML that is a mapping at the top level, not a list of plays.** The file that proves the "list of mappings" test is what identifies a playbook — a reader that treats every `.yml` as a playbook fails here. |
| `scripts/rotate-certs.sh` | Not Ansible. Ignored. |

## `broken/` — one instance of each rule

Every rule of Requirement 9.2 has exactly one subject here, so a test can assert a rule fires
**and** assert nothing else does.

| File | What it guards |
|---|---|
| `site.yml` | `ansible.dangling-import` — `import_playbook: does-not-exist.yml`. |
| `playbooks/deploy.yml` (play 1) | `ansible.role-missing` — `absent-role` has no folder. |
| `playbooks/deploy.yml` (play 2) | `ansible.unmatched-hosts` — `hosts: cache`, and the inventory defines only `web`. |
| `playbooks/deploy.yml` (play 3) | **Not a problem.** `roles: ["{{ role_name }}"]` is unresolvable, not missing. A rule that reported this would fire on every parameterised role in every real repository, which is why the resolution state is three-valued (Requirement 3.4). |
| `roles/web/tasks/main.yml` | `ansible.dangling-import` in its second form — `include_tasks: tls.yml` with no such file in the role. |
| `roles/web/meta/main.yml` | `ansible.role-missing` in its second form — a **dependency** naming a role with no folder. |
| `roles/hollow/README.md` | `ansible.empty-role` — a folder under `roles/` with no canonical subfolder at all. The README is there only because git cannot store an empty directory; it is not canonical content, so the role is still hollow. |
| `unparsable.yml` | `ansible.unreadable-yaml` — an unclosed quote and an unbalanced bracket. The problem must carry **the parser's own message and line**, not ADP's paraphrase, and the rest of the tree must still read (Requirement 1.2). |

## `ini-inventory/` — a root inventory in INI

The smallest tree with the inventory format most real projects and every old tutorial use: a
classic INI `hosts` at the folder root, read as INI because it is not YAML, and reported by nothing.
Added with the fix that stopped the unmatched-hosts rule claiming that no inventory defines groups
this file plainly defines (`2b6a3f1da`).

| File | What it guards |
|---|---|
| `hosts` | `[web]` with two hosts, `[db]` with one, `[app:children]` gathering both (three hosts, one level deep) and `[web:vars]`, which is configuration rather than membership and adds no group. |
| `site.yml` | One play with `hosts: web`, which the INI groups resolve, so `ansible.unmatched-hosts` stays silent. |

## `unconventional/` — valid Ansible, and the diagram says nothing

The fixture for Requirement 9.4: *"an unregistered convention is not a problem, merely
undrawn"*. This tree is one a real team could deploy from — it simply keeps nothing where the
recommended layout puts it, and says so in its own `ansible.cfg`.

| File | What it guards |
|---|---|
| `ansible.cfg` | The **only** thing recognised here. Its presence is what makes this a stronger test than an empty folder: the reader ran, found the annotation, and still had nothing to complain about. |
| `plays/site.yml` | A valid playbook in a folder the model does not recognise (not root, not `playbooks/`). Undrawn, not reported. |
| `automation/roles/deploy/**` | A valid role under a custom `roles_path`. Undrawn, not reported — and specifically **not** reported as an empty role, since it is not under `roles/` at all. |
| `environments/prod/**` | A valid inventory with `group_vars`, in a folder the model does not recognise. Undrawn, not reported. |

**The assertion this tree exists for is `Assert.Empty(problems)`.** If a rule ever fires here,
the rule has started guessing about layouts rather than judging the one it recognises.

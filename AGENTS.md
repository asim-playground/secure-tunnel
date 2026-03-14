# AGENTS.md - Repository Guide for Large-Language-Model Agents

Our goal is an easy to use, easy to maintain project with reproducible local workflows.

Use plan tool to track tasks. Use jujutsu (`jj`) for version control. Use independent sub-agents as needed.
If available in this Codex session, use the `$jujutsu` skill for command semantics.

Use `jj` to look at previous few commits to get a sense of recent changes. Analyze recently created open tasks to get a sense of planned work. If there is duplication of work between open tasks, think about how to refactor and consolidate the tasks.

- Fast iteration: `mise run dev` (wire this task to format + lint + test for this repo).
- Keep work scoped to backlog tasks with clear acceptance criteria.
- Keep task implementation details and verification steps current while coding.

## Backlog plans and tasks

Backlog is local-only by default via `.git/info/exclude` (`backlog/`).
If you choose to track backlog files in Git, remove that local exclude and keep task references auditable in PRs.

When creating a new plan or task, once you think you are finished, iteratively review with an independent sub-agent and make fix-ups and re-reviews until both of you are satisfied.

### Plans

A plan is a broader idea that is broken down into tasks. Plans are stored in
`backlog/plans/`.

Plans should use the template in `backlog/plans/_template.md`.

### Tasks

A task is a specific, actionable item that is part of a plan. Tasks are stored
in `backlog/tasks/`.

- Each commit/PR should reference a task ID from `backlog/tasks/task-<eight-digit-id>_<title>.md` (for example `task-00000042_fix-retry-loop.md`).
- Do not start implementing before converting an idea into a task with clear Acceptance Criteria.
- Tasks must be atomic, independent, and testable.
- New tasks must follow the template in `backlog/tasks/_template.md`.
- While implementing, update the task's Implementation Plan.
- After a task is complete:
  1. ensure `mise run dev` passes.
  2. Use an independent sub-agent and iteratively review the changes made for the task until you both agree on the changes and are satisfied with the result.
  3. add Implementation Notes and tick all Acceptance Criteria.
  4. Keep the parent plan up to date; add/remove/edit tasks and update Markdown checklists.
  5. move it into `backlog/tasks/completed/`.
  6. use jujutsu (`jj`) to push to remote and run `jj new` to ensure you are on a new empty commit.

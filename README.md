# pushkind-dantes

This README is intentionally minimal.

## Documentation Map

- `SPEC.md`: product specification and current implementation contract.
- `AGENTS.md`: contributor and code-generation guidelines for architecture, coding standards, and verification commands.

## Development Workflow

- Start from `SPEC.md`; all implementation work must align with it.
- For each new work item, add `specs/features/<name>.md` and `plans/<name>.md`.
- For architecture changes, add or update an ADR in `specs/decisions/`.

## Repository-Specific Pointers

- `pushkind-dantes.service`: example systemd unit for running this service.
- `LICENSE`: project license.

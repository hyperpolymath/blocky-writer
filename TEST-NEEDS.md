# TEST-NEEDS.md — blocky-writer

## CRG Grade: C — ACHIEVED 2026-04-04

## Current Test State

| Category | Count | Notes |
|----------|-------|-------|
| Test directories | 1 | Location(s): /tests |
| CI workflows | 15 | Running tests on GitHub Actions |
| Unit tests | Configured | ReScript Jest/Vitest setup |

## What's Covered

- [x] ReScript unit tests
- [x] JavaScript interop tests

## Still Missing (for CRG B+)

- [ ] Code coverage reports (codecov integration)
- [ ] Detailed test documentation in CONTRIBUTING.md
- [ ] Integration tests beyond unit tests
- [ ] Performance benchmarking suite

## Run Tests

```bash
npm run test  # or: rescript build && npm run test
```

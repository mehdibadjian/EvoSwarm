# GEMINI.md — Model Instructions & Guidelines

Guidelines for Gemini models operating in this repository.

## Engineering Standards
- **Strict TDD Discipline**: Write failing tests first. Only write production code to make tests pass.
- **Zero AI Slop**: Do not include conversational remarks, issue tracking headers, or LLM metadata in source code comments.
- **Verification Before Trust**: Never claim a task or story is complete without executing the test suite and checking git diffs.
- **Clickable Links**: In chat responses, always format file paths and symbols as clickable markdown links (`[file.py](file:///path/to/file.py)`).
- **Interactive Questions**: Use the native `ask_question` tool for design clarifications rather than dumping terminal menu prompts.

"""e1-13 benchmark harness package.

The harness compares EvoSwarm against a single-shot-with-feedback baseline at
equal token budgets (Gate 1). Live execution against a model provider is the
SEAM half (roadmap §6 requires LLM keys) and is not exercised in this crate's
test suite; the deterministic halves — fixture integrity, structural
fairness invariants and solve-rate/gate math — are the shipped surface.
"""

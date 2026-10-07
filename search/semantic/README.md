# Optional semantic search

Design boundary only; there is no model dependency or executable semantic crate.
A future implementation should accept a bounded local candidate set, use explicitly
installed local models, expose memory/latency budgets, and unload models when idle.
Never upload queries, history, embeddings or private-session content.

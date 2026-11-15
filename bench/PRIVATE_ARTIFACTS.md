# Benchmark artifact policy

Commit synthetic fixtures, test code, scripts, sanitized metric summaries and
build hashes. Keep model weights, transcripts, local paths, SQLite databases,
server logs and raw run artifacts local. `bench/runs/` is ignored. A sanitized
summary supports review but does not replace the raw samples for reproduction.
Model download manifests belong beside the local model, not in the repository.

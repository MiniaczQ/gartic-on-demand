# Complex sequences

## Upsert player

1. Try update player name
2. If failed, create player

## Play from round N = 0

1. Try to allocate one unallocated round 0
2. If failed, create a pre-allocated round 0

## Play from round N > 0

1. Try find first round N without direct attempts, ordered by least previous approved attempts, then by random.
2. If failed, no further rounds available.

## Complete round

1. Set attempt as accepted.
2.1. Clone round, update date, incremet round, set multiplex from mode logic.
2.2. Attach previous attempts.
2.3. Add accepted attempt as previous attempt.

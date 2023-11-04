# Definitions

## Allocation

Each round can be allocated `multiplex` times by attempts.
Round with `multiplex` allocations can no longer be allocated.
Attempts that are `Accepted` are permanent allocations, while other attempts from the allocating group can result in deallocaiton.

## User participation

User can participate in a round in two ways:
- Direct participation
- Previous participation

### Direct participation

User attempt is allocating the round directly.
Users cannot directly participate in one round multiple times.
It is however, possible to allocate the round, unallocate it (expiry, cancellation, rejection) and allocate it again.

### Previous participation

User attempt was accepted as one of the round predecessors.
Users can participate in rounds they previously participated in, but rounds where they participated the least are always prefered.

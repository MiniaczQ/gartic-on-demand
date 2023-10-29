# Definitions

## Allocated round
A round is allocated once per each session that is approved or can lead to approval.
The limit of allocations per round is dictated by the `multiplex` field.

## User previously participated
User has previously participated in a round if he is listed in previous sessions.

## User participated
User has participated in a round if one of his sessions is currently allocating said round.

# Database layout

```mermaid
erDiagram
    User ||--o{ Session: ""
    User ||--o{ PreviousSessions: ""
    Round ||--|{ Session: ""
    Round ||--o{ PreviousSessions: ""

    User {
        u64 id
        String name
        DateTime created_at
    }
    Session {
        Id id
        SessionState state
        DateTime created_at
    }
    PreviousSessions {
        Id id
        DateTime created_at
    }
    Round {
        Id id
        Mode mode
        bool nsfw
        u64 round
        u64 multiplex
        DateTime created_at
    }
```

# Session state

```mermaid
flowchart TD;
    Active --> Cancelled
    Active --> TimedOut
    Active --> Uploading
    Uploading --> Pending
    Pending --> Rejected
    Pending --> Approved
    Uploading --> Approved
```

# Complex sequences:

## Upsert player

1. Try update player name
2. If failed, create player

## Play from round N = 0

1. Try to allocate one unallocated round 0
2. If failed, create a pre-allocated round 0

## Play from round N > 0

User cannot play the exact same round twice, but if necessary, they can participate in a round they were in previously.

1. Try find round N where user did not participate nor previously participate.
2. If failed, try find round N where user did not participate, but could have previously participated.
3. If failed, no further rounds available.

## Complete round



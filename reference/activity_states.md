```mermaid
flowchart TD;
    Inactive -- activity --> Active
    Active -- inactivity --> Cooldown
    Cooldown -- time + inactivity --> Inactive
    Cooldown -- activity --> Cooldown
```
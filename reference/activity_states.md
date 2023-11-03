```mermaid
flowchart TD;
    Inactive -- activity --> Active
    Active -- inactivity --> Cooldown
    Cooldown -- time --> Inactive
    Cooldown -- activity --> Cooldown
```
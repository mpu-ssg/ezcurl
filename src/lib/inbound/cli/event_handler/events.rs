#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliEvent {
    Insert(InsertEvent),
    Normal(NormalEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalEvent {
    EnterInsert,
    Move(Direction),
    Quit,
    ToggleHistory,
    ToggleLeaderKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertEvent {
    Backspace,
    Delete,
    ExitInsert,
    InsertChar(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

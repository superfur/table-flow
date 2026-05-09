//! `TableStateMachine` —— 处理 `TableEvent` 并产出 `StateTransition`。

use std::collections::VecDeque;

use tf_core::{StateTransition, TableEvent, TableId, TfError};

use crate::state::TableState;

const EVENT_LOG_CAPACITY: usize = 1000;

pub struct TableStateMachine {
    pub table_id: TableId,
    state: TableState,
    event_log: VecDeque<TableEvent>,
    action_seq: u32,
    hand_seq: u64,
}

impl TableStateMachine {
    pub fn new(table_id: TableId) -> Self {
        let state = TableState::initial(table_id.clone());
        Self {
            table_id,
            state,
            event_log: VecDeque::with_capacity(EVENT_LOG_CAPACITY),
            action_seq: 0,
            hand_seq: 0,
        }
    }

    /// 处理一个事件，返回该事件触发的状态转移列表。
    /// TODO(detail-impl):
    ///   - NewHandDetected → reset seats / phase / pot
    ///   - HoleCardsDetected → 写入 state.hole_cards + bump confidence
    ///   - CommunityCardsChanged → 推进 street + reset current_bet
    ///   - ActionReconstructed → 调用 self.handle_action()
    ///   - PotChanged / SeatStatusChanged / DealerButtonMoved → 更新对应字段
    ///   - Timeout → 衰减 confidence
    /// 末尾把 event 推入 event_log（环形缓冲，上限 EVENT_LOG_CAPACITY）
    pub fn process_event(&mut self, _event: TableEvent) -> Result<Vec<StateTransition>, TfError> {
        todo!("TableStateMachine::process_event")
    }

    /// 推进 current_player_turn 到下一个 Active 玩家
    /// TODO(detail-impl)
    pub fn advance_turn(&mut self) {
        todo!("TableStateMachine::advance_turn")
    }

    /// 重置到 Waiting 阶段（recovery 路径调用）
    /// TODO(detail-impl)
    pub fn reset_to_waiting(&mut self) {
        todo!("TableStateMachine::reset_to_waiting")
    }

    pub fn state(&self) -> &TableState {
        &self.state
    }

    pub fn snapshot(&self) -> TableState {
        self.state.clone()
    }

    pub fn event_log(&self) -> &VecDeque<TableEvent> {
        &self.event_log
    }

    pub fn next_action_seq(&mut self) -> u32 {
        self.action_seq += 1;
        self.action_seq
    }

    pub fn next_hand_seq(&mut self) -> u64 {
        self.hand_seq += 1;
        self.hand_seq
    }
}

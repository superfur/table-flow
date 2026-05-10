//! `TableStateMachine` —— 处理 `TableEvent` 并产出 `StateTransition`。

use std::collections::VecDeque;

use tf_core::{
    ActionType, BlindKind, Card, SeatId, SeatStatus, StateTransition, Street, TableEvent,
    TableId, TablePhase, TfError,
};

use crate::state::{ActionRecord, PotInfo, TableState};

const EVENT_LOG_CAPACITY: usize = 1000;
const EPS: f64 = 0.01;

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

    pub fn process_event(&mut self, event: TableEvent) -> Result<Vec<StateTransition>, TfError> {
        let transitions = match &event {
            TableEvent::NewHandDetected { dealer_seat } => {
                self.handle_new_hand(*dealer_seat)
            }
            TableEvent::HoleCardsDetected { cards } => {
                self.state.hole_cards = Some(*cards);
                self.bump_confidence(0.05);
                vec![]
            }
            TableEvent::CommunityCardsChanged { cards, street } => {
                self.handle_community_change(cards.clone(), *street)
            }
            TableEvent::ActionReconstructed { action } => {
                self.handle_action(action.clone())
            }
            TableEvent::PotChanged { new_total, delta } => {
                let _ = delta;
                self.state.pot.total = *new_total;
                self.state.pot.main_pot = *new_total;
                vec![]
            }
            TableEvent::SeatStatusChanged { seat_id, new_status } => {
                if let Some(seat) = self.state.seats.iter_mut().find(|s| s.seat_id == *seat_id) {
                    seat.status = *new_status;
                }
                vec![]
            }
            TableEvent::DealerButtonMoved { new_seat } => {
                self.state.dealer_seat = Some(*new_seat);
                vec![]
            }
            TableEvent::Timeout => {
                self.state.state_confidence = (self.state.state_confidence - 0.05).max(0.5);
                vec![StateTransition::StateConfidenceUpdated {
                    new_confidence: self.state.state_confidence,
                }]
            }
        };

        self.push_event_log(event);
        Ok(transitions)
    }

    fn handle_new_hand(&mut self, dealer_seat: SeatId) -> Vec<StateTransition> {
        self.hand_seq += 1;
        self.action_seq = 0;

        self.state.phase = TablePhase::Playing;
        self.state.street = Street::Preflop;
        self.state.hand_number = self.hand_seq;
        self.state.dealer_seat = Some(dealer_seat);
        self.state.hole_cards = None;
        self.state.community_cards.clear();
        self.state.pot = PotInfo::default();
        self.state.action_history.clear();
        self.state.current_player_turn = None;

        for seat in &mut self.state.seats {
            seat.current_bet = 0.0;
            seat.total_bet_this_hand = 0.0;
            seat.last_action = None;
            seat.has_cards = true;
            if seat.status == SeatStatus::Folded {
                seat.status = SeatStatus::Active;
            }
        }

        vec![StateTransition::HandStarted {
            hand_number: self.hand_seq,
        }]
    }

    fn handle_community_change(&mut self, cards: Vec<Card>, new_street: Street) -> Vec<StateTransition> {
        let old_street = self.state.street;
        self.state.community_cards = cards;
        self.state.street = new_street;

        for seat in &mut self.state.seats {
            seat.current_bet = 0.0;
        }
        self.state.blinds.current_max_bet = 0.0;
        self.state.blinds.last_raise_size = self.state.blinds.big_blind;

        vec![StateTransition::StreetChanged {
            from: old_street,
            to: new_street,
        }]
    }

    fn handle_action(&mut self, action: tf_core::ReconstructedAction) -> Vec<StateTransition> {
        self.action_seq += 1;
        let seq = self.action_seq;
        let amount = action.amount.unwrap_or(0.0);

        let record = ActionRecord {
            seat_id: action.seat_id,
            action: action.action_type.clone(),
            amount,
            street: action.street,
            seq,
            confidence: action.confidence,
        };

        if let Some(seat) = self
            .state
            .seats
            .iter_mut()
            .find(|s| s.seat_id == action.seat_id)
        {
            match &action.action_type {
                ActionType::Fold => {
                    seat.status = SeatStatus::Folded;
                    seat.has_cards = false;
                }
                ActionType::Check => {}
                ActionType::Call => {
                    seat.stack -= amount;
                    seat.current_bet += amount;
                    seat.total_bet_this_hand += amount;
                }
                ActionType::Bet(bet_amount) => {
                    seat.stack -= bet_amount;
                    seat.current_bet += bet_amount;
                    seat.total_bet_this_hand += bet_amount;
                    self.state.blinds.last_raise_size = *bet_amount;
                    self.state.blinds.current_max_bet = seat.current_bet;
                }
                ActionType::Raise(total) => {
                    let raise_amount = *total - seat.current_bet;
                    let raise_increment = *total - self.state.blinds.current_max_bet;
                    seat.stack -= raise_amount;
                    seat.current_bet = *total;
                    seat.total_bet_this_hand += raise_amount;
                    self.state.blinds.last_raise_size =
                        raise_increment.max(self.state.blinds.big_blind);
                    self.state.blinds.current_max_bet = *total;
                }
                ActionType::AllIn(_total) => {
                    let allin_amount = seat.stack;
                    seat.stack = 0.0;
                    seat.current_bet += allin_amount;
                    seat.total_bet_this_hand += allin_amount;
                    seat.status = SeatStatus::AllIn;
                    if seat.current_bet > self.state.blinds.current_max_bet + EPS {
                        self.state.blinds.current_max_bet = seat.current_bet;
                    }
                }
                ActionType::PostBlind(kind) => {
                    seat.stack -= amount;
                    seat.current_bet += amount;
                    seat.total_bet_this_hand += amount;
                    match kind {
                        BlindKind::SmallBlind => {
                            self.state.blinds.small_blind = amount;
                        }
                        BlindKind::BigBlind => {
                            self.state.blinds.big_blind = amount;
                            self.state.blinds.current_max_bet = amount;
                            self.state.blinds.last_raise_size = amount;
                        }
                        BlindKind::Straddle => {
                            self.state.blinds.straddle = amount;
                            self.state.blinds.current_max_bet = amount;
                        }
                        BlindKind::Ante => {
                            self.state.blinds.ante = amount;
                            seat.current_bet -= amount;
                        }
                    }
                }
            }
            seat.last_action = Some(record.clone());
        }

        self.advance_turn();

        self.state.action_history.push(record.clone());
        vec![StateTransition::ActionRecorded(tf_core::ActionRecordSummary {
            seat_id: record.seat_id,
            action: record.action,
            amount: record.amount,
            street: record.street,
            seq: record.seq,
            confidence: record.confidence,
        })]
    }

    pub fn advance_turn(&mut self) {
        let n = self.state.seats.len();
        if n == 0 {
            return;
        }
        let start = self
            .state
            .current_player_turn
            .map(|s| s.0 as usize)
            .unwrap_or(0);
        for i in 1..=n {
            let idx = (start + i) % n;
            let seat = &self.state.seats[idx];
            if matches!(seat.status, SeatStatus::Active) {
                self.state.current_player_turn = Some(seat.seat_id);
                return;
            }
        }
        self.state.current_player_turn = None;
    }

    pub fn reset_to_waiting(&mut self) {
        self.state.phase = TablePhase::Waiting;
        self.state.street = Street::Preflop;
        self.state.hand_number = 0;
        self.state.dealer_seat = None;
        self.state.hole_cards = None;
        self.state.community_cards.clear();
        self.state.pot = PotInfo::default();
        self.state.action_history.clear();
        self.state.current_player_turn = None;
        self.state.blinds = tf_core::BlindsInfo::default();
        self.action_seq = 0;
        for seat in &mut self.state.seats {
            seat.current_bet = 0.0;
            seat.total_bet_this_hand = 0.0;
            seat.last_action = None;
            seat.has_cards = false;
        }
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

    fn push_event_log(&mut self, event: TableEvent) {
        if self.event_log.len() >= EVENT_LOG_CAPACITY {
            self.event_log.pop_front();
        }
        self.event_log.push_back(event);
    }

    fn bump_confidence(&mut self, delta: f32) {
        self.state.state_confidence = (self.state.state_confidence + delta).min(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SeatState;
    use tf_core::{ActionSource, Card, Rank, Suit, ReconstructedAction};

    fn make_machine_with_seats() -> TableStateMachine {
        let mut sm = TableStateMachine::new("test".to_string());
        sm.state.seats = (0..6u8)
            .map(|i| SeatState {
                seat_id: SeatId::new(i),
                status: if i < 4 {
                    SeatStatus::Active
                } else {
                    SeatStatus::Empty
                },
                stack: 100.0,
                current_bet: 0.0,
                total_bet_this_hand: 0.0,
                last_action: None,
                is_hero: i == 0,
                has_cards: i < 4,
            })
            .collect();
        sm
    }

    #[test]
    fn test_new_hand() {
        let mut sm = make_machine_with_seats();
        let transitions = sm
            .process_event(TableEvent::NewHandDetected {
                dealer_seat: SeatId::new(2),
            })
            .unwrap();

        assert_eq!(transitions.len(), 1);
        assert!(matches!(
            &transitions[0],
            StateTransition::HandStarted { hand_number: 1 }
        ));
        assert_eq!(sm.state().phase, TablePhase::Playing);
        assert_eq!(sm.state().street, Street::Preflop);
        assert_eq!(sm.state().dealer_seat, Some(SeatId::new(2)));
        assert!(sm.state().hole_cards.is_none());
        assert!(sm.state().community_cards.is_empty());
        assert!(sm.state().action_history.is_empty());
    }

    #[test]
    fn test_hole_cards_detected() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();

        let cards = [
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
                confidence: 0.99,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
                confidence: 0.99,
            },
        ];
        sm.process_event(TableEvent::HoleCardsDetected { cards })
            .unwrap();

        assert_eq!(sm.state().hole_cards, Some(cards));
    }

    #[test]
    fn test_community_cards_changed() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();

        let flop = vec![
            Card {
                suit: Suit::Spades,
                rank: Rank::Two,
                confidence: 0.99,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Three,
                confidence: 0.99,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Four,
                confidence: 0.99,
            },
        ];
        let transitions = sm
            .process_event(TableEvent::CommunityCardsChanged {
                cards: flop.clone(),
                street: Street::Flop,
            })
            .unwrap();

        assert_eq!(transitions.len(), 1);
        assert!(matches!(
            &transitions[0],
            StateTransition::StreetChanged {
                from: Street::Preflop,
                to: Street::Flop
            }
        ));
        assert_eq!(sm.state().community_cards, flop);
        assert_eq!(sm.state().street, Street::Flop);
    }

    #[test]
    fn test_fold_action() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();

        let transitions = sm
            .process_event(TableEvent::ActionReconstructed {
                action: ReconstructedAction {
                    seat_id: SeatId::new(1),
                    action_type: ActionType::Fold,
                    amount: None,
                    street: Street::Preflop,
                    timestamp_ms: 0,
                    confidence: 0.95,
                    source: ActionSource::SeatStatusChange,
                },
            })
            .unwrap();

        assert_eq!(transitions.len(), 1);
        assert!(matches!(
            &transitions[0],
            StateTransition::ActionRecorded(_)
        ));
        assert_eq!(sm.state().seats[1].status, SeatStatus::Folded);
        assert!(!sm.state().seats[1].has_cards);
    }

    #[test]
    fn test_call_action() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();
        sm.state.blinds.big_blind = 2.0;
        sm.state.blinds.current_max_bet = 2.0;

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(0),
                action_type: ActionType::Call,
                amount: Some(2.0),
                street: Street::Preflop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        assert!((sm.state().seats[0].stack - 98.0).abs() < 0.01);
        assert!((sm.state().seats[0].current_bet - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_bet_action() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();
        sm.state.blinds.big_blind = 2.0;
        sm.state.blinds.current_max_bet = 0.0;
        sm.state.street = Street::Flop;

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(0),
                action_type: ActionType::Bet(6.0),
                amount: Some(6.0),
                street: Street::Flop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        assert!((sm.state().seats[0].stack - 94.0).abs() < 0.01);
        assert!((sm.state().seats[0].current_bet - 6.0).abs() < 0.01);
        assert!((sm.state().blinds.current_max_bet - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_raise_action() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();
        sm.state.blinds.big_blind = 2.0;
        sm.state.blinds.current_max_bet = 2.0;

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(0),
                action_type: ActionType::Raise(8.0),
                amount: Some(8.0),
                street: Street::Preflop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        assert!((sm.state().seats[0].stack - 92.0).abs() < 0.01);
        assert!((sm.state().seats[0].current_bet - 8.0).abs() < 0.01);
        assert!((sm.state().blinds.current_max_bet - 8.0).abs() < 0.01);
        assert!((sm.state().blinds.last_raise_size - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_allin_action() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();
        sm.state.blinds.big_blind = 2.0;
        sm.state.blinds.current_max_bet = 2.0;

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(0),
                action_type: ActionType::AllIn(100.0),
                amount: Some(100.0),
                street: Street::Preflop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        assert!((sm.state().seats[0].stack).abs() < 0.01);
        assert_eq!(sm.state().seats[0].status, SeatStatus::AllIn);
    }

    #[test]
    fn test_post_blind_sb() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(1),
                action_type: ActionType::PostBlind(BlindKind::SmallBlind),
                amount: Some(1.0),
                street: Street::Preflop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        assert!((sm.state().seats[1].stack - 99.0).abs() < 0.01);
        assert!((sm.state().seats[1].current_bet - 1.0).abs() < 0.01);
        assert!((sm.state().blinds.small_blind - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_post_blind_bb() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(2),
                action_type: ActionType::PostBlind(BlindKind::BigBlind),
                amount: Some(2.0),
                street: Street::Preflop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        assert!((sm.state().seats[2].stack - 98.0).abs() < 0.01);
        assert!((sm.state().blinds.big_blind - 2.0).abs() < 0.01);
        assert!((sm.state().blinds.current_max_bet - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_advance_turn() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();
        sm.state.current_player_turn = Some(SeatId::new(0));

        sm.advance_turn();
        assert_eq!(sm.state().current_player_turn, Some(SeatId::new(1)));
    }

    #[test]
    fn test_advance_turn_skips_folded() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();
        sm.state.seats[1].status = SeatStatus::Folded;
        sm.state.current_player_turn = Some(SeatId::new(0));

        sm.advance_turn();
        assert_eq!(sm.state().current_player_turn, Some(SeatId::new(2)));
    }

    #[test]
    fn test_reset_to_waiting() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();

        sm.reset_to_waiting();

        assert_eq!(sm.state().phase, TablePhase::Waiting);
        assert_eq!(sm.state().hand_number, 0);
        assert!(sm.state().dealer_seat.is_none());
    }

    #[test]
    fn test_event_log_capacity() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();

        for _ in 0..1100 {
            sm.process_event(TableEvent::Timeout).unwrap();
        }

        assert!(sm.event_log().len() <= EVENT_LOG_CAPACITY);
    }

    #[test]
    fn test_timeout_decreases_confidence() {
        let mut sm = make_machine_with_seats();
        let initial_conf = sm.state().state_confidence;

        sm.process_event(TableEvent::Timeout).unwrap();

        assert!(sm.state().state_confidence < initial_conf);
        assert!(sm.state().state_confidence >= 0.5);
    }

    #[test]
    fn test_dealer_button_moved() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::DealerButtonMoved {
            new_seat: SeatId::new(3),
        })
        .unwrap();

        assert_eq!(sm.state().dealer_seat, Some(SeatId::new(3)));
    }

    #[test]
    fn test_pot_changed() {
        let mut sm = make_machine_with_seats();
        sm.process_event(TableEvent::PotChanged {
            new_total: 42.0,
            delta: 10.0,
        })
        .unwrap();

        assert!((sm.state().pot.total - 42.0).abs() < 0.01);
    }

    #[test]
    fn test_full_hand_lifecycle() {
        let mut sm = make_machine_with_seats();

        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(1),
                action_type: ActionType::PostBlind(BlindKind::SmallBlind),
                amount: Some(1.0),
                street: Street::Preflop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(2),
                action_type: ActionType::PostBlind(BlindKind::BigBlind),
                amount: Some(2.0),
                street: Street::Preflop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(0),
                action_type: ActionType::Call,
                amount: Some(2.0),
                street: Street::Preflop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        sm.process_event(TableEvent::ActionReconstructed {
            action: ReconstructedAction {
                seat_id: SeatId::new(1),
                action_type: ActionType::Call,
                amount: Some(1.0),
                street: Street::Preflop,
                timestamp_ms: 0,
                confidence: 0.9,
                source: ActionSource::StackDiff,
            },
        })
        .unwrap();

        assert_eq!(sm.state().action_history.len(), 4);
        assert_eq!(sm.state().hand_number, 1);
    }
}

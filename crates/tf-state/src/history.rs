//! 手牌历史记录 —— 数据模型 + JSONL 持久化。
//!
//! 每手牌开始时创建 `HandRecord`，随状态机事件逐步填充，
//! 手牌结束时（`HandCompleted` / 新手牌开始）刷盘。

use std::io::{BufWriter, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use tf_core::{BlindsInfo, Card, SeatId, StateTransition, Street, TableEvent, TableId};

use crate::state::ActionRecord;

// ─── 数据模型 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandRecord {
    pub hand_id: u64,
    pub table_id: TableId,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub blinds: BlindsInfo,
    pub dealer_seat: Option<SeatId>,
    pub hero_seat: Option<SeatId>,
    pub hole_cards: Option<[Card; 2]>,
    pub community_cards: Vec<Card>,
    pub pot_total: f64,
    pub actions: Vec<ActionRecord>,
    pub recommendations: Vec<RecommendationEntry>,
    pub result: Option<HandResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationEntry {
    pub action_seq: u32,
    pub recommendation: RecSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecSnapshot {
    pub action: String,
    pub amount: f64,
    pub confidence: f64,
    pub distribution: std::collections::HashMap<String, f64>,
    pub ev: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandResult {
    pub winners: Vec<WinnerInfo>,
    pub pot_awarded: f64,
    pub rake: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinnerInfo {
    pub seat_id: SeatId,
    pub amount: f64,
    pub hand_description: Option<String>,
}

impl HandRecord {
    pub fn new(hand_id: u64, table_id: TableId, started_at_ms: i64) -> Self {
        Self {
            hand_id,
            table_id,
            started_at_ms,
            ended_at_ms: None,
            blinds: BlindsInfo::default(),
            dealer_seat: None,
            hero_seat: None,
            hole_cards: None,
            community_cards: Vec::new(),
            pot_total: 0.0,
            actions: Vec::new(),
            recommendations: Vec::new(),
            result: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.ended_at_ms.is_some()
    }
}

// ─── JSONL 持久化 ───────────────────────────────────────────────────────

pub struct HandHistoryWriter {
    writer: BufWriter<std::fs::File>,
    count: u64,
}

impl HandHistoryWriter {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            count: 0,
        })
    }

    pub fn append(&mut self, record: &HandRecord) -> std::io::Result<()> {
        let mut line = serde_json::to_string(record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push('\n');
        self.writer.write_all(line.as_bytes())?;
        self.writer.flush()?;
        self.count += 1;
        Ok(())
    }

    pub fn count(&self) -> u64 {
        self.count
    }
}

pub fn read_hand_history(path: &Path) -> std::io::Result<Vec<HandRecord>> {
    let content = std::fs::read_to_string(path)?;
    let mut records = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<HandRecord>(line) {
            Ok(r) => records.push(r),
            Err(e) => {
                tracing::warn!("Skipping malformed hand history line: {}", e);
            }
        }
    }
    Ok(records)
}

// ─── 会话统计 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_hands: u64,
    pub hands_with_hero: u64,
    pub hero_wins: u64,
    pub hero_net: f64,
    pub vpip_count: u64,
    pub pfr_count: u64,
    pub total_pot: f64,
    pub biggest_pot: f64,
    pub street_distribution: std::collections::HashMap<String, u64>,
}

impl SessionStats {
    pub fn from_records(records: &[HandRecord]) -> Self {
        let mut stats = Self::default();
        stats.total_hands = records.len() as u64;

        for rec in records {
            stats.total_pot += rec.pot_total;
            if rec.pot_total > stats.biggest_pot {
                stats.biggest_pot = rec.pot_total;
            }

            let street_name = format!("{:?}", rec.community_cards.len());
            *stats
                .street_distribution
                .entry(street_name)
                .or_insert(0) += 1;

            let hero_seat = match rec.hero_seat {
                Some(s) => s,
                None => continue,
            };

            stats.hands_with_hero += 1;

            let hero_voluntary = rec.actions.iter().any(|a| {
                a.seat_id == hero_seat
                    && !matches!(
                        a.action,
                        tf_core::ActionType::PostBlind(_)
                            | tf_core::ActionType::Check
                            | tf_core::ActionType::Fold
                    )
            });
            if hero_voluntary {
                stats.vpip_count += 1;
            }

            let hero_pfr = rec.actions.iter().any(|a| {
                a.seat_id == hero_seat
                    && a.street == Street::Preflop
                    && matches!(
                        a.action,
                        tf_core::ActionType::Bet(_) | tf_core::ActionType::Raise(_)
                    )
            });
            if hero_pfr {
                stats.pfr_count += 1;
            }

            if let Some(ref result) = rec.result {
                for w in &result.winners {
                    if w.seat_id == hero_seat {
                        stats.hero_wins += 1;
                        stats.hero_net += w.amount;
                    }
                }
            }
        }

        stats
    }

    pub fn vpip(&self) -> f64 {
        if self.hands_with_hero == 0 {
            return 0.0;
        }
        self.vpip_count as f64 / self.hands_with_hero as f64 * 100.0
    }

    pub fn pfr(&self) -> f64 {
        if self.hands_with_hero == 0 {
            return 0.0;
        }
        self.pfr_count as f64 / self.hands_with_hero as f64 * 100.0
    }

    pub fn win_rate(&self) -> f64 {
        if self.hands_with_hero == 0 {
            return 0.0;
        }
        self.hero_wins as f64 / self.hands_with_hero as f64 * 100.0
    }
}

// ─── HandHistoryRecorder ────────────────────────────────────────────────

pub struct HandHistoryRecorder {
    current: Option<HandRecord>,
    hand_seq: u64,
    table_id: TableId,
}

impl HandHistoryRecorder {
    pub fn new(table_id: TableId) -> Self {
        Self {
            current: None,
            hand_seq: 0,
            table_id,
        }
    }

    pub fn on_event(
        &mut self,
        event: &TableEvent,
        transitions: &[StateTransition],
        state: &crate::state::TableState,
    ) {
        for t in transitions {
            if let StateTransition::HandStarted { hand_number } = t {
                self.finalize_current();
                self.hand_seq = *hand_number;
                let mut rec = HandRecord::new(
                    self.hand_seq,
                    self.table_id.clone(),
                    chrono_now_ms(),
                );
                rec.blinds = state.blinds.clone();
                rec.dealer_seat = state.dealer_seat;
                rec.hero_seat = state.hero_seat;
                self.current = Some(rec);
                return;
            }
        }

        let rec = match &mut self.current {
            Some(r) => r,
            None => return,
        };

        match event {
            TableEvent::HoleCardsDetected { cards } => {
                rec.hole_cards = Some(*cards);
                rec.hero_seat = state.hero_seat;
            }
            TableEvent::CommunityCardsChanged { cards, .. } => {
                rec.community_cards = cards.clone();
            }
            TableEvent::PotChanged { new_total, .. } => {
                rec.pot_total = *new_total;
            }
            TableEvent::DealerButtonMoved { new_seat } => {
                rec.dealer_seat = Some(*new_seat);
            }
            _ => {}
        }

        for t in transitions {
            if let StateTransition::ActionRecorded(summary) = t {
                if let Some(ar) = state.action_history.iter().find(|a| a.seq == summary.seq) {
                    rec.actions.push(ar.clone());
                }
            }
        }
    }

    pub fn on_transition(&mut self, transition: &StateTransition) {
        let rec = match &mut self.current {
            Some(r) => r,
            None => return,
        };

        if let StateTransition::ActionRecorded(summary) = transition {
            if rec.actions.len() < summary.seq as usize {
                tracing::debug!(
                    "Action seq {} not yet in rec.actions (len={})",
                    summary.seq,
                    rec.actions.len()
                );
            }
        }
    }

    pub fn add_recommendation(&mut self, action_seq: u32, rec_snapshot: RecSnapshot) {
        if let Some(rec) = &mut self.current {
            rec.recommendations
                .push(RecommendationEntry {
                    action_seq,
                    recommendation: rec_snapshot,
                });
        }
    }

    pub fn finalize_current(&mut self) -> Option<HandRecord> {
        let mut rec = self.current.take()?;
        if rec.ended_at_ms.is_none() {
            rec.ended_at_ms = Some(chrono_now_ms());
        }
        Some(rec)
    }

    pub fn current(&self) -> Option<&HandRecord> {
        self.current.as_ref()
    }
}

fn chrono_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ─── 测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tf_core::{ActionType, BlindsInfo, Rank, Suit};

    fn make_test_hand(hand_id: u64, with_hero: bool) -> HandRecord {
        let mut rec = HandRecord::new(hand_id, "test-table".to_string(), 1000);
        rec.blinds = BlindsInfo {
            small_blind: 1.0,
            big_blind: 2.0,
            ..Default::default()
        };
        rec.dealer_seat = Some(SeatId::new(0));
        if with_hero {
            rec.hero_seat = Some(SeatId::new(2));
            rec.hole_cards = Some([
                Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.99 },
                Card { suit: Suit::Hearts, rank: Rank::King, confidence: 0.98 },
            ]);
        }
        rec.pot_total = 30.0;
        rec.ended_at_ms = Some(5000);
        rec
    }

    #[test]
    fn test_hand_record_new() {
        let rec = HandRecord::new(1, "t1".to_string(), 100);
        assert_eq!(rec.hand_id, 1);
        assert!(!rec.is_complete());
        assert!(rec.actions.is_empty());
    }

    #[test]
    fn test_hand_record_complete() {
        let rec = make_test_hand(1, true);
        assert!(rec.is_complete());
        assert_eq!(rec.started_at_ms, 1000);
        assert_eq!(rec.ended_at_ms, Some(5000));
    }

    #[test]
    fn test_jsonl_roundtrip() {
        let dir = std::env::temp_dir().join("tf-history-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.jsonl");

        let mut writer = HandHistoryWriter::open(&path).unwrap();

        let mut rec1 = make_test_hand(1, true);
        rec1.actions.push(ActionRecord {
            seat_id: SeatId::new(2),
            action: ActionType::Call,
            amount: 2.0,
            street: Street::Preflop,
            seq: 1,
            confidence: 0.95,
        });
        writer.append(&rec1).unwrap();

        let rec2 = make_test_hand(2, false);
        writer.append(&rec2).unwrap();

        assert_eq!(writer.count(), 2);

        let loaded = read_hand_history(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].hand_id, 1);
        assert_eq!(loaded[0].actions.len(), 1);
        assert_eq!(loaded[1].hand_id, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_session_stats_empty() {
        let stats = SessionStats::from_records(&[]);
        assert_eq!(stats.total_hands, 0);
        assert_eq!(stats.vpip(), 0.0);
        assert_eq!(stats.pfr(), 0.0);
    }

    #[test]
    fn test_session_stats_with_hero() {
        let mut rec = make_test_hand(1, true);
        rec.actions.push(ActionRecord {
            seat_id: SeatId::new(2),
            action: ActionType::Call,
            amount: 2.0,
            street: Street::Preflop,
            seq: 1,
            confidence: 0.95,
        });
        rec.actions.push(ActionRecord {
            seat_id: SeatId::new(2),
            action: ActionType::Bet(6.0),
            amount: 6.0,
            street: Street::Flop,
            seq: 2,
            confidence: 0.9,
        });
        rec.result = Some(HandResult {
            winners: vec![WinnerInfo {
                seat_id: SeatId::new(2),
                amount: 30.0,
                hand_description: Some("pair of aces".to_string()),
            }],
            pot_awarded: 30.0,
            rake: 1.5,
        });

        let stats = SessionStats::from_records(&[rec]);
        assert_eq!(stats.total_hands, 1);
        assert_eq!(stats.hands_with_hero, 1);
        assert_eq!(stats.hero_wins, 1);
        assert!((stats.hero_net - 30.0).abs() < 0.01);
        assert_eq!(stats.vpip_count, 1);
        assert_eq!(stats.pfr_count, 0);
        assert!((stats.vpip() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_session_stats_pfr() {
        let mut rec = make_test_hand(1, true);
        rec.actions.push(ActionRecord {
            seat_id: SeatId::new(2),
            action: ActionType::Raise(6.0),
            amount: 6.0,
            street: Street::Preflop,
            seq: 1,
            confidence: 0.95,
        });

        let stats = SessionStats::from_records(&[rec]);
        assert_eq!(stats.vpip_count, 1);
        assert_eq!(stats.pfr_count, 1);
        assert!((stats.pfr() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_session_stats_no_hero() {
        let rec = make_test_hand(1, false);
        let stats = SessionStats::from_records(&[rec]);
        assert_eq!(stats.total_hands, 1);
        assert_eq!(stats.hands_with_hero, 0);
        assert_eq!(stats.hero_wins, 0);
    }

    #[test]
    fn test_read_malformed_skips() {
        let dir = std::env::temp_dir().join("tf-history-malformed");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.jsonl");

        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "not json").unwrap();
        writeln!(f).unwrap();
        let rec = make_test_hand(1, true);
        let good = serde_json::to_string(&rec).unwrap();
        writeln!(f, "{}", good).unwrap();

        let loaded = read_hand_history(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].hand_id, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_recorder_full_hand() {
        let mut sm = crate::machine::TableStateMachine::new("test-rec".to_string());
        let mut recorder = HandHistoryRecorder::new("test-rec".to_string());

        let event = TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(1),
        };
        let transitions = sm.process_event(event.clone()).unwrap();
        recorder.on_event(&event, &transitions, sm.state());

        assert!(recorder.current().is_some());
        assert_eq!(recorder.current().unwrap().hand_id, 1);

        let event = TableEvent::HoleCardsDetected {
            cards: [
                Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.99 },
                Card { suit: Suit::Hearts, rank: Rank::King, confidence: 0.98 },
            ],
        };
        let transitions = sm.process_event(event.clone()).unwrap();
        recorder.on_event(&event, &transitions, sm.state());
        assert_eq!(recorder.current().unwrap().hole_cards.unwrap()[0].rank, Rank::Ace);

        let event = TableEvent::PotChanged {
            new_total: 30.0,
            delta: 30.0,
        };
        let transitions = sm.process_event(event.clone()).unwrap();
        recorder.on_event(&event, &transitions, sm.state());
        assert!((recorder.current().unwrap().pot_total - 30.0).abs() < 0.01);

        let completed = recorder.finalize_current();
        assert!(completed.is_some());
        assert!(completed.unwrap().is_complete());
        assert!(recorder.current().is_none());
    }

    #[test]
    fn test_recorder_two_hands_auto_finalize() {
        let mut sm = crate::machine::TableStateMachine::new("test-multi".to_string());
        let mut recorder = HandHistoryRecorder::new("test-multi".to_string());

        let event = TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        };
        let transitions = sm.process_event(event.clone()).unwrap();
        recorder.on_event(&event, &transitions, sm.state());

        assert!(recorder.current().is_some());

        let event2 = TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(1),
        };
        let transitions2 = sm.process_event(event2.clone()).unwrap();
        recorder.on_event(&event2, &transitions2, sm.state());

        assert!(recorder.current().is_some());
        assert_eq!(recorder.current().unwrap().hand_id, 2);
    }
}

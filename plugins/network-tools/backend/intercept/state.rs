//! 拦截器共享状态：环形缓冲、WS 记录、配置、事件出口。

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tokio::sync::mpsc::UnboundedSender;

use super::dto::{now_ms, FlowRecord, FlowSummary, ProxyConfig, WsFrame, WsRecord, WsSummary};

/// 待发送给前端的批量事件项
pub enum Outbox {
    Flow(FlowSummary),
    Ws(WsSummary),
}

pub struct SharedState {
    pub config: Mutex<ProxyConfig>,
    flows: Mutex<VecDeque<FlowRecord>>,
    ws: Mutex<HashMap<u64, WsRecord>>,
    ws_keys: Mutex<HashMap<String, u64>>,
    seq: AtomicU64,
    outbox: Mutex<Option<UnboundedSender<Outbox>>>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            config: Mutex::new(ProxyConfig::default()),
            flows: Mutex::new(VecDeque::new()),
            ws: Mutex::new(HashMap::new()),
            ws_keys: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
            outbox: Mutex::new(None),
        }
    }
}

impl SharedState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_id(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn set_outbox(&self, tx: Option<UnboundedSender<Outbox>>) {
        if let Ok(mut guard) = self.outbox.lock() {
            *guard = tx;
        }
    }

    pub fn emit(&self, message: Outbox) {
        if let Ok(guard) = self.outbox.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(message);
            }
        }
    }

    fn max_flows(&self) -> usize {
        self.config
            .lock()
            .map(|c| c.max_flows)
            .unwrap_or_else(|_| ProxyConfig::default().max_flows)
    }

    /// 追加完成流量，返回摘要（用于事件）
    pub fn push_flow(&self, record: FlowRecord) -> FlowSummary {
        let summary = record.summary.clone();
        let max = self.max_flows().max(1);
        if let Ok(mut queue) = self.flows.lock() {
            queue.push_back(record);
            while queue.len() > max {
                queue.pop_front();
            }
        }
        summary
    }

    pub fn flows_list(&self) -> Vec<FlowSummary> {
        self.flows
            .lock()
            .map(|queue| queue.iter().map(|record| record.summary.clone()).collect())
            .unwrap_or_default()
    }

    pub fn flow_count(&self) -> usize {
        self.flows.lock().map(|queue| queue.len()).unwrap_or(0)
    }

    pub fn flow_get(&self, id: u64) -> Option<FlowRecord> {
        self.flows
            .lock()
            .ok()
            .and_then(|queue| queue.iter().find(|record| record.summary.id == id).cloned())
    }

    pub fn clear_flows(&self) {
        if let Ok(mut queue) = self.flows.lock() {
            queue.clear();
        }
    }

    pub fn ws_list(&self) -> Vec<WsSummary> {
        self.ws
            .lock()
            .map(|map| map.values().map(|record| record.summary.clone()).collect())
            .unwrap_or_default()
    }

    pub fn ws_count(&self) -> usize {
        self.ws.lock().map(|map| map.len()).unwrap_or(0)
    }

    pub fn ws_get(&self, id: u64) -> Option<WsRecord> {
        self.ws.lock().ok().and_then(|map| map.get(&id).cloned())
    }

    pub fn clear_ws(&self) {
        if let Ok(mut map) = self.ws.lock() {
            map.clear();
        }
        if let Ok(mut keys) = self.ws_keys.lock() {
            keys.clear();
        }
    }

    /// 追加 WS 帧（按连接 key 归组），返回摘要
    pub fn ws_push(&self, key: String, host: String, url: String, frame: WsFrame, max: usize) -> WsSummary {
        let id = match self.ws_keys.lock() {
            Ok(mut keys) => *keys.entry(key).or_insert_with(|| self.next_id()),
            Err(_) => self.next_id(),
        };
        let mut map = match self.ws.lock() {
            Ok(guard) => guard,
            Err(_) => return WsSummary {
                id,
                host,
                url,
                frame_count: 1,
                started_at: now_ms(),
                truncated: false,
            },
        };
        let entry = map.entry(id).or_insert_with(|| WsRecord {
            summary: WsSummary {
                id,
                host,
                url,
                frame_count: 0,
                started_at: now_ms(),
                truncated: false,
            },
            frames: Vec::new(),
        });
        entry.summary.frame_count += 1;
        if !entry.summary.truncated {
            if entry.frames.len() < max.max(1) {
                entry.frames.push(frame);
            } else {
                entry.summary.truncated = true;
            }
        }
        entry.summary.clone()
    }
}

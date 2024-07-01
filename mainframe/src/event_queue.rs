use crate::event::Attendance;
use sol_util::mainframe::{Event, EventJsonBody};
use std::io::Write;
use std::{fs, io::Read, sync::Arc};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Clone)]
pub struct EventQueue {
    queue: Vec<EventJsonBody>,
}

impl EventQueue {
    pub fn new() -> Self {
        let mut f = fs::OpenOptions::new()
            .read(true)
            .create(true)
            .truncate(false)
            .write(true)
            .open("event_queue.ron")
            .expect("unable to open event queue");

        let mut buf = String::new();
        f.read_to_string(&mut buf)
            .expect("failed to read queue file");

        let queue: Vec<EventJsonBody> = ron::from_str(&buf).unwrap_or(Vec::new());
        Self { queue }
    }

    fn write(&mut self) {
        let str = ron::to_string(&self.queue).expect("should convert to a str");
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open("event_queue.ron")
            .expect("unable to open event queue");

        f.write_all(str.as_bytes())
            .expect("unable to write to the event queue")
    }

    pub fn push(&mut self, event: EventJsonBody) {
        self.queue.push(event);
        self.write();
    }

    pub fn pop(&mut self) -> Option<EventJsonBody> {
        if let Some(e) = self.queue.pop() {
            self.write();
            return Some(e);
        }

        None
    }
}

async fn write_to_db(event: Event, db_url: String, db_token: String) {
    let conn = crate::get_db_conn(db_url, db_token).await.unwrap();

    let conn_arc = Arc::new(conn.clone());
    let mut unlogged = event.log_attendance(conn_arc.clone()).await;
    if !unlogged.is_empty() {
        let mut tries = 0;
        loop {
            if !unlogged.is_empty() {
                break;
            }
            if tries > 5 {
                eprintln!("failed to log {unlogged:?}");
                break;
            }
            let fake_event = Event {
                host: event.host,
                attendance: unlogged,
                kind: event.kind.clone(),
                location: event.location.clone(),
                metadata: event.metadata.clone(),
                event_date: event.event_date,
            };
            unlogged = fake_event.log_attendance(conn_arc.clone()).await;
            tries += 1;
        }
    }

    let attendance_string = serde_json::to_string(&event.attendance).unwrap();
    conn.execute("INSERT INTO events (host, attendance, event_date, kind, location) VALUES (?1, ?2, ?3, ?4, ?5)", (
        event.host,
        attendance_string,
        event.event_date.to_rfc3339(),
        event.kind.as_str(),
        event.location.as_str(),
    )).await.unwrap();
}

// can get stuck in one of these for a while
pub async fn process_event(event: EventJsonBody) -> Event {
    let mut names = event.names.clone();
    let mut attendees = Vec::new();
    loop {
        let attendees_map = sol_util::roblox::get_user_ids_from_usernames(&names)
            .await
            .expect("should get ids from roblox");

        let mut found_all = true;
        let mut new_names = Vec::new();
        for (i, (_rank_name, id_opt)) in attendees_map.into_iter().enumerate() {
            match id_opt {
                Some(id) => attendees.push(id),
                None => {
                    new_names.push(names[i].clone());
                    found_all = false
                }
            }
        }

        if found_all {
            break;
        } else {
            names = new_names;
        }

        sleep(Duration::from_secs(1)).await;
    }

    if attendees.len() != names.len() {
        eprintln!("attendees does not match names");
    }
    Event::new(event.host, attendees, event.location, event.kind)
}

pub fn queue_loop(
    queue: Arc<Mutex<EventQueue>>,
    db_url: String,
    db_token: String,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await;
            let mut q = queue.lock().await;

            if let Some(event) = q.pop() {
                drop(q); // we drop the lock, otherwise everything might halt
                println!("Popped queue event {event:?}");
                let processed = process_event(event).await;
                write_to_db(processed, db_url.clone(), db_token.clone()).await;
            }
        }
    })
}

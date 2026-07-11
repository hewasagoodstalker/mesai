use chrono::{DateTime, Datelike, Days, Local, NaiveDate, Utc};
use eframe::egui;
use egui::{Color32, RichText};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};

const APP_ID: &str = "mesai";

// ---------------------------------------------------------------------------
// Localization
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
enum Lang {
    En,
    Tr,
}

impl Default for Lang {
    fn default() -> Self {
        Lang::En
    }
}

/// Every fixed UI string, in one place per language. Using a struct (instead
/// of a string map) means a missing translation is a compile error.
struct Strings {
    tab_timer: &'static str,
    tab_sessions: &'static str,
    tab_stats: &'static str,
    tab_export: &'static str,

    running: &'static str,
    paused: &'static str,
    ready: &'static str,

    start: &'static str,
    idle_tip: &'static str,
    started_label: &'static str,
    ended_label: &'static str,
    duration_label: &'static str,

    resume: &'static str,
    pause: &'static str,
    finish_save: &'static str,
    cancel: &'static str,
    discard_running_q: &'static str,
    yes_discard: &'static str,
    no_keep: &'static str,

    session_attachments: &'static str,
    no_attachments_yet: &'static str,
    remove: &'static str,
    path_hint: &'static str,
    add: &'static str,
    add_files: &'static str,
    picker_title: &'static str,

    save_session: &'static str,
    topic_label: &'static str,
    topic_hint: &'static str,
    notes_label: &'static str,
    notes_hint: &'static str,
    tags_label: &'static str,
    tags_hint: &'static str,
    attachments_label: &'static str,
    save: &'static str,
    discard: &'static str,
    sure_q: &'static str,
    yes_dont_save: &'static str,

    no_sessions_1: &'static str,
    no_sessions_2: &'static str,
    no_topic: &'static str,
    tags_colon: &'static str,
    notes_colon: &'static str,
    attachments_colon: &'static str,
    open: &'static str,
    open_attach_dir: &'static str,
    delete: &'static str,
    delete_q: &'static str,
    yes_delete: &'static str,

    stats_empty: &'static str,
    total_time: &'static str,
    session_count: &'static str,
    avg_session: &'static str,
    streak_label: &'static str,
    active_days: &'static str,
    streak_suffix: &'static str,
    last_14: &'static str,
    by_topic: &'static str,

    export_heading: &'static str,
    export_csv_info: &'static str,
    export_json_info: &'static str,
    export_csv_btn: &'static str,
    export_json_btn: &'static str,
    last_export_label: &'static str,
    open_folder: &'static str,
    data_dir_label: &'static str,
    data_dir_info: &'static str,

    timer_started: &'static str,
    timer_paused_msg: &'static str,
    resumed: &'static str,
    finish_fill: &'static str,
    discarded_msg: &'static str,
    saved_msg: &'static str,
    deleted_msg: &'static str,
    start_first: &'static str,
    type_path_first: &'static str,
    added_prefix: &'static str,
    not_found_prefix: &'static str,
    csv_written: &'static str,
    json_written: &'static str,
    export_err: &'static str,
    save_err: &'static str,

    unit_h: &'static str,
    unit_m: &'static str,
    unit_s: &'static str,
}

static STR_EN: Strings = Strings {
    tab_timer: "Timer",
    tab_sessions: "Sessions",
    tab_stats: "Stats",
    tab_export: "Export",

    running: "Running",
    paused: "Paused",
    ready: "Ready.",

    start: "Start",
    idle_tip: "Tip: while the timer is running, attach the files you're working on\nwith 'Add files…'. (Drag & drop also works, but only on X11.)",
    started_label: "Started:",
    ended_label: "Ended:",
    duration_label: "Duration:",

    resume: "Resume",
    pause: "Pause",
    finish_save: "Finish & save",
    cancel: "Cancel",
    discard_running_q: "Discard without saving?",
    yes_discard: "Yes, discard",
    no_keep: "No, keep it",

    session_attachments: "Attachments for this session",
    no_attachments_yet: "No attachments yet — click 'Add files…' or type a path below.",
    remove: "Remove",
    path_hint: "File path (e.g. ~/project/main.c)",
    add: "Add",
    add_files: "Add files…",
    picker_title: "Choose files to attach",

    save_session: "Save session",
    topic_label: "Topic",
    topic_hint: "e.g. C — Milestone 2: tokenizing with strtok",
    notes_label: "Notes",
    notes_hint: "What did you do, where did you get stuck, where will you pick up next time?",
    tags_label: "Tags (comma-separated)",
    tags_hint: "e.g. c, shell, pointers",
    attachments_label: "Attachments",
    save: "Save",
    discard: "Discard",
    sure_q: "Are you sure?",
    yes_dont_save: "Yes, discard",

    no_sessions_1: "No sessions yet.",
    no_sessions_2: "Start your first one from the Timer tab!",
    no_topic: "(no topic)",
    tags_colon: "Tags:",
    notes_colon: "Notes:",
    attachments_colon: "Attachments:",
    open: "Open",
    open_attach_dir: "Open attachments folder",
    delete: "Delete",
    delete_q: "Delete this session and its files?",
    yes_delete: "Yes, delete",

    stats_empty: "Record a few sessions first — your stats will show up here.",
    total_time: "Total time:",
    session_count: "Sessions:",
    avg_session: "Average session:",
    streak_label: "Streak:",
    active_days: "Active days:",
    streak_suffix: "days in a row",
    last_14: "Last 14 days",
    by_topic: "By topic",

    export_heading: "Export",
    export_csv_info: "CSV: a summary table for spreadsheet apps (LibreOffice Calc etc.).",
    export_json_info: "JSON: a full backup of all data, attachment paths included.",
    export_csv_btn: "Export CSV",
    export_json_btn: "Export JSON",
    last_export_label: "Last export:",
    open_folder: "Open folder",
    data_dir_label: "Data folder:",
    data_dir_info: "Sessions live in data.json; attachment copies are stored under attachments/.",

    timer_started: "Timer started. Good luck!",
    timer_paused_msg: "Timer paused.",
    resumed: "Resumed.",
    finish_fill: "Session finished — fill in the details and save.",
    discarded_msg: "Session discarded.",
    saved_msg: "Session saved.",
    deleted_msg: "Session deleted.",
    start_first: "Start the timer first — attachments are tied to a session.",
    type_path_first: "Type a file path first.",
    added_prefix: "Added:",
    not_found_prefix: "File not found:",
    csv_written: "CSV written:",
    json_written: "JSON written:",
    export_err: "Export failed:",
    save_err: "Save error:",

    unit_h: "h",
    unit_m: "min",
    unit_s: "s",
};

static STR_TR: Strings = Strings {
    tab_timer: "Sayaç",
    tab_sessions: "Kayıtlar",
    tab_stats: "İstatistik",
    tab_export: "Dışa Aktar",

    running: "Çalışıyor",
    paused: "Duraklatıldı",
    ready: "Hazır.",

    start: "Başlat",
    idle_tip: "İpucu: Sayaç çalışırken üzerinde çalıştığın dosyaları 'Dosya ekle…' ile\noturuma iliştirebilirsin. (Sürükle-bırak yalnızca X11'de çalışır.)",
    started_label: "Başlangıç:",
    ended_label: "Bitiş:",
    duration_label: "Süre:",

    resume: "Devam et",
    pause: "Duraklat",
    finish_save: "Bitir ve kaydet",
    cancel: "İptal",
    discard_running_q: "Kaydedilmeden silinsin mi?",
    yes_discard: "Evet, sil",
    no_keep: "Vazgeç",

    session_attachments: "Bu oturumun ekleri",
    no_attachments_yet: "Henüz ek yok — 'Dosya ekle…' düğmesini kullan ya da aşağıya yolunu yaz.",
    remove: "Kaldır",
    path_hint: "Dosya yolu (ör. ~/proje/main.c)",
    add: "Ekle",
    add_files: "Dosya ekle…",
    picker_title: "Eklenecek dosyaları seç",

    save_session: "Oturumu kaydet",
    topic_label: "Konu",
    topic_hint: "ör. C — Milestone 2: strtok ile tokenizasyon",
    notes_label: "Notlar",
    notes_hint: "Bugün ne yaptın, nerede takıldın, bir dahaki sefere nereden devam edeceksin?",
    tags_label: "Etiketler (virgülle ayır)",
    tags_hint: "ör. c, shell, pointer",
    attachments_label: "Ekler",
    save: "Kaydet",
    discard: "Kaydetmeden sil",
    sure_q: "Emin misin?",
    yes_dont_save: "Evet, kaydetme",

    no_sessions_1: "Henüz kayıt yok.",
    no_sessions_2: "Sayaç sekmesinden ilk çalışmanı başlat!",
    no_topic: "(konu yok)",
    tags_colon: "Etiketler:",
    notes_colon: "Not:",
    attachments_colon: "Ekler:",
    open: "Aç",
    open_attach_dir: "Ek klasörünü aç",
    delete: "Sil",
    delete_q: "Kayıt ve ekleri silinsin mi?",
    yes_delete: "Evet, sil",

    stats_empty: "İstatistik için önce birkaç oturum kaydetmen gerekiyor.",
    total_time: "Toplam süre:",
    session_count: "Oturum sayısı:",
    avg_session: "Ortalama oturum:",
    streak_label: "Seri:",
    active_days: "Farklı gün:",
    streak_suffix: "gün üst üste",
    last_14: "Son 14 gün",
    by_topic: "Konulara göre",

    export_heading: "Dışa aktar",
    export_csv_info: "CSV: tablo programlarında (LibreOffice Calc vb.) açmak için özet tablo.",
    export_json_info: "JSON: tüm verinin tam yedeği; ek dosyaların yolları da içinde.",
    export_csv_btn: "CSV olarak dışa aktar",
    export_json_btn: "JSON olarak dışa aktar",
    last_export_label: "Son dışa aktarma:",
    open_folder: "Klasörü aç",
    data_dir_label: "Veri klasörü:",
    data_dir_info: "Oturumlar data.json içinde; ek kopyaları attachments/ altında saklanır.",

    timer_started: "Sayaç başladı. Kolay gelsin!",
    timer_paused_msg: "Sayaç duraklatıldı.",
    resumed: "Devam ediliyor.",
    finish_fill: "Oturum bitti — bilgileri doldurup kaydet.",
    discarded_msg: "Oturum kaydedilmedi.",
    saved_msg: "Oturum kaydedildi.",
    deleted_msg: "Kayıt silindi.",
    start_first: "Önce sayacı başlat — ekler bir oturuma bağlanır.",
    type_path_first: "Önce bir dosya yolu yaz.",
    added_prefix: "Eklendi:",
    not_found_prefix: "Dosya bulunamadı:",
    csv_written: "CSV yazıldı:",
    json_written: "JSON yazıldı:",
    export_err: "Dışa aktarma hatası:",
    save_err: "Kaydetme hatası:",

    unit_h: "sa",
    unit_m: "dk",
    unit_s: "sn",
};

const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_TR: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran",
"Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık",
];
const DAYS_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const DAYS_TR: [&str; 7] = ["Pzt", "Sal", "Çar", "Per", "Cum", "Cts", "Paz"];

impl Lang {
    fn s(self) -> &'static Strings {
        match self {
            Lang::En => &STR_EN,
            Lang::Tr => &STR_TR,
        }
    }

    fn months(self) -> &'static [&'static str; 12] {
        match self {
            Lang::En => &MONTHS_EN,
            Lang::Tr => &MONTHS_TR,
        }
    }

    fn days(self) -> &'static [&'static str; 7] {
        match self {
            Lang::En => &DAYS_EN,
            Lang::Tr => &DAYS_TR,
        }
    }

    /// "Jul 11, 2026, 14:30" / "11 Temmuz 2026, 14:30" (local time)
    fn dt(self, dt: &DateTime<Utc>) -> String {
        let l = dt.with_timezone(&Local);
        let month = self.months()[l.month0() as usize];
        let hm = l.format("%H:%M");
        match self {
            Lang::En => format!("{} {}, {}, {}", month, l.day(), l.year(), hm),
            Lang::Tr => format!("{} {} {}, {}", l.day(), month, l.year(), hm),
        }
    }

    /// "Sat, Jul 11" / "Cts 11 Tem"
    fn date_short(self, d: NaiveDate) -> String {
        let wd = self.days()[d.weekday().num_days_from_monday() as usize];
        let month = self.months()[d.month0() as usize];
        let mon3: String = month.chars().take(3).collect();
        match self {
            Lang::En => format!("{}, {} {}", wd, mon3, d.day()),
            Lang::Tr => format!("{} {} {}", wd, d.day(), mon3),
        }
    }

    /// "1 h 25 min" / "1 sa 25 dk"
    fn dur_human(self, total: i64) -> String {
        let s = self.s();
        let t = total.max(0);
        let (h, m, sec) = (t / 3600, (t % 3600) / 60, t % 60);
        if h > 0 {
            format!("{} {} {} {}", h, s.unit_h, m, s.unit_m)
        } else if m > 0 {
            format!("{} {} {} {}", m, s.unit_m, sec, s.unit_s)
        } else {
            format!("{} {}", sec, s.unit_s)
        }
    }

    fn sessions_summary(self, n: usize, dur: &str) -> String {
        match self {
            Lang::En => format!("{n} sessions • {dur} total"),
            Lang::Tr => format!("Toplam {n} oturum • {dur}"),
        }
    }

    fn attached_n(self, n: usize) -> String {
        match self {
            Lang::En => format!("{n} file(s) attached."),
            Lang::Tr => format!("{n} ek eklendi."),
        }
    }

    fn saved_failed(self, failed: usize) -> String {
        match self {
            Lang::En => format!("Session saved ({failed} attachment(s) could not be copied)."),
            Lang::Tr => format!("Oturum kaydedildi ({failed} ek kopyalanamadı)."),
        }
    }

    fn streak_line(self, n: u32) -> String {
        format!("{} {}", n, self.s().streak_suffix)
    }
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
struct Attachment {
    file_name: String,
    path: PathBuf, // path of the copy under attachments/<id>/
}

#[derive(Serialize, Deserialize, Clone)]
struct Session {
    id: u64,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    /// Active work time (pauses excluded), in seconds.
    duration_secs: u64,
    topic: String,
    notes: String,
    tags: Vec<String>,
    attachments: Vec<Attachment>,
}

#[derive(Serialize, Deserialize, Default)]
struct Database {
    next_id: u64,
    /// UI language; defaults to English for files written by older versions.
    #[serde(default)]
    lang: Lang,
    sessions: Vec<Session>,
}

struct Paths {
    data_dir: PathBuf,
    db_file: PathBuf,
    attach_dir: PathBuf,
}

fn app_paths() -> Paths {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")); // ~/.local/share
    let data_dir = base.join(APP_ID);
    Paths {
        db_file: data_dir.join("data.json"),
        attach_dir: data_dir.join("attachments"),
        data_dir,
    }
}

fn load_db(paths: &Paths) -> Database {
    fs::read_to_string(&paths.db_file)
    .ok()
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_default()
}

/// Write to a temp file first, then rename: if writing is interrupted the
/// old data stays intact.
fn save_db(paths: &Paths, db: &Database) -> std::io::Result<()> {
    fs::create_dir_all(&paths.data_dir)?;
    let tmp = paths.db_file.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(db).expect("could not serialize data");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &paths.db_file)
}

/// Copy the source file under attachments/<id>/. If a file with the same
/// name exists, append _1, _2, ...
fn copy_attachment(attach_dir: &Path, id: u64, src: &Path) -> std::io::Result<Attachment> {
    let dir = attach_dir.join(id.to_string());
    fs::create_dir_all(&dir)?;

    let name = src
    .file_name()
    .map(|s| s.to_string_lossy().into_owned())
    .unwrap_or_else(|| "attachment".to_string());

    let mut dest = dir.join(&name);
    let mut n = 1;
    while dest.exists() {
        let stem = Path::new(&name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment");
        let new_name = match Path::new(&name).extension().and_then(|s| s.to_str()) {
            Some(ext) => format!("{stem}_{n}.{ext}"),
            None => format!("{stem}_{n}"),
        };
        dest = dir.join(new_name);
        n += 1;
    }

    fs::copy(src, &dest)?;
    Ok(Attachment {
        file_name: dest.file_name().unwrap().to_string_lossy().into_owned(),
       path: dest,
    })
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn fmt_hms(total: i64) -> String {
    let t = total.max(0);
    format!("{:02}:{:02}:{:02}", t / 3600, (t % 3600) / 60, t % 60)
}

fn parse_tags(s: &str) -> Vec<String> {
    s.split(',')
    .map(|t| t.trim())
    .filter(|t| !t.is_empty())
    .map(|t| t.to_string())
    .collect()
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}

/// Validate a hand-typed path and push it to the list; returns the status
/// message to show.
fn add_manual_path(list: &mut Vec<PathBuf>, input: &mut String, lang: Lang) -> String {
    let s = lang.s();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return s.type_path_first.to_string();
    }
    let p = expand_tilde(trimmed);
    if p.is_file() {
        let msg = format!("{} {}", s.added_prefix, p.display());
        list.push(p);
        input.clear();
        msg
    } else {
        format!("{} {}", s.not_found_prefix, p.display())
    }
}

/// Open a file or folder with the system default application.
fn open_in_system(p: &Path) {
    let _ = Command::new("xdg-open")
    .arg(p)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn();
}

fn csv_field(s: &str) -> String {
    if s.contains(&[',', ';', '"', '\n', '\r'][..]) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn export_csv(db: &Database, dir: &Path) -> std::io::Result<PathBuf> {
    let path = dir.join(format!("mesai_{}.csv", Local::now().format("%Y%m%d_%H%M%S")));
    let mut out =
    String::from("id,started,ended,duration_seconds,duration,topic,tags,notes,attachments\n");
    for s in &db.sessions {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            s.id,
            s.started_at.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
                              s.ended_at.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
                              s.duration_secs,
                              csv_field(&db.lang.dur_human(s.duration_secs as i64)),
                              csv_field(&s.topic),
                              csv_field(&s.tags.join(", ")),
                              csv_field(&s.notes),
                              s.attachments.len(),
        ));
    }
    fs::write(&path, out)?;
    Ok(path)
}

fn export_json(db: &Database, dir: &Path) -> std::io::Result<PathBuf> {
    let path = dir.join(format!("mesai_{}.json", Local::now().format("%Y%m%d_%H%M%S")));
    fs::write(&path, serde_json::to_string_pretty(db).unwrap())?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// Timer and draft (a finished session waiting to be saved)
// ---------------------------------------------------------------------------

/// Time is measured with the wall clock instead of Instant, so the elapsed
/// time stays correct even if the laptop suspends and resumes.
struct RunningTimer {
    started_at: DateTime<Utc>,
    seg_start: DateTime<Utc>, // last "resume" moment
    accumulated_secs: i64,    // total of previous segments
    paused: bool,
}

impl RunningTimer {
    fn new() -> Self {
        let now = Utc::now();
        Self {
            started_at: now,
            seg_start: now,
            accumulated_secs: 0,
            paused: false,
        }
    }
    fn elapsed_secs(&self) -> i64 {
        let extra = if self.paused {
            0
        } else {
            (Utc::now() - self.seg_start).num_seconds().max(0)
        };
        self.accumulated_secs + extra
    }
    fn pause(&mut self) {
        if !self.paused {
            self.accumulated_secs = self.elapsed_secs();
            self.paused = true;
        }
    }
    fn resume(&mut self) {
        if self.paused {
            self.seg_start = Utc::now();
            self.paused = false;
        }
    }
}

/// The form shown after the timer is stopped.
struct Draft {
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    duration_secs: u64,
    topic: String,
    notes: String,
    tags: String,
    attachments: Vec<PathBuf>,
    discard_armed: bool, // two-click confirmation for "discard"
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Timer,
    Sessions,
    Stats,
    Export,
}

struct MesaiApp {
    paths: Paths,
    db: Database,
    tab: Tab,

    timer: Option<RunningTimer>,
    pending_attachments: Vec<PathBuf>, // attachments collected while the timer runs
    draft: Option<Draft>,

    manual_path: String,  // text box for typing a path by hand
    status: String,       // status line at the bottom
    cancel_armed: bool,   // two-click cancel for a running timer
    confirm_delete: Option<u64>,
    last_export: Option<PathBuf>,

    // Native file picker (runs on its own thread so the UI never blocks;
    // picked paths come back through this channel).
    picker_tx: Sender<Vec<PathBuf>>,
    picker_rx: Receiver<Vec<PathBuf>>,
    picker_busy: bool,
}

impl MesaiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let paths = app_paths();
        let db = load_db(&paths);
        let (picker_tx, picker_rx) = channel();
        let status = db.lang.s().ready.to_string();
        Self {
            paths,
            db,
            tab: Tab::Timer,
            timer: None,
            pending_attachments: Vec::new(),
            draft: None,
            manual_path: String::new(),
            status,
            cancel_armed: false,
            confirm_delete: None,
            last_export: None,
            picker_tx,
            picker_rx,
            picker_busy: false,
        }
    }

    fn save(&mut self) {
        if let Err(e) = save_db(&self.paths, &self.db) {
            self.status = format!("{} {e}", self.db.lang.s().save_err);
        }
    }

    /// Open the native file picker on a background thread. On Wayland this
    /// goes through the XDG desktop portal, i.e. your desktop's own dialog.
    fn open_file_picker(&mut self, ctx: egui::Context) {
        if self.picker_busy {
            return;
        }
        self.picker_busy = true;
        let tx = self.picker_tx.clone();
        let title = self.db.lang.s().picker_title;
        std::thread::spawn(move || {
            let files = pollster::block_on(
                rfd::AsyncFileDialog::new().set_title(title).pick_files(),
            );
            let paths: Vec<PathBuf> = files
            .map(|fs| fs.iter().map(|f| f.path().to_path_buf()).collect())
            .unwrap_or_default();
            let _ = tx.send(paths);
            ctx.request_repaint(); // wake the UI so the new files show up immediately
        });
    }

    /// New attachment paths (from the picker or drag & drop) go to the draft
    /// if one is open, otherwise to the running timer's pending list.
    fn route_new_attachments(&mut self, paths: Vec<PathBuf>) {
        let lang = self.db.lang;
        let n = paths.len();
        if let Some(d) = self.draft.as_mut() {
            d.attachments.extend(paths);
            self.status = lang.attached_n(n);
        } else if self.timer.is_some() {
            self.pending_attachments.extend(paths);
            self.status = lang.attached_n(n);
        } else {
            self.status = lang.s().start_first.to_string();
        }
    }

    /// Turn the draft into a permanent session: copy attachments, write the DB.
    fn persist_draft(&mut self, d: Draft) {
        let lang = self.db.lang;
        let id = self.db.next_id.max(1);
        self.db.next_id = id + 1;

        let mut attachments = Vec::new();
        let mut failed = 0usize;
        for p in &d.attachments {
            match copy_attachment(&self.paths.attach_dir, id, p) {
                Ok(a) => attachments.push(a),
                Err(_) => failed += 1,
            }
        }

        self.db.sessions.push(Session {
            id,
            started_at: d.started_at,
            ended_at: d.ended_at,
            duration_secs: d.duration_secs,
            topic: d.topic.trim().to_string(),
                              notes: d.notes.trim().to_string(),
                              tags: parse_tags(&d.tags),
                              attachments,
        });
        self.db.sessions.sort_by_key(|s| s.started_at);
        self.save();

        self.status = if failed == 0 {
            lang.s().saved_msg.to_string()
        } else {
            lang.saved_failed(failed)
        };
    }

    // -- Tabs ------------------------------------------------------------

    fn ui_timer(&mut self, ui: &mut egui::Ui) {
        if self.draft.is_some() {
            self.ui_draft_form(ui);
            return;
        }

        let lang = self.db.lang;
        let s = lang.s();

        #[derive(PartialEq)]
        enum Act {
            None,
            Start,
            Pause,
            Resume,
            Finish,
            CancelArm,
            CancelUnarm,
            CancelDo,
        }
        let mut act = Act::None;
        let mut want_picker = false;

        match &self.timer {
            None => {
                ui.add_space(36.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("00:00:00").monospace().size(56.0).weak());
                    ui.add_space(16.0);
                    let btn = egui::Button::new(RichText::new(s.start).size(18.0))
                    .min_size(egui::vec2(180.0, 44.0));
                    if ui.add(btn).clicked() {
                        act = Act::Start;
                    }
                    ui.add_space(20.0);
                    ui.label(s.idle_tip);
                });
            }
            Some(t) => {
                let elapsed = t.elapsed_secs();
                let (state_txt, state_color) = if t.paused {
                    (s.paused, Color32::from_rgb(230, 190, 90))
                } else {
                    (s.running, Color32::from_rgb(130, 200, 130))
                };

                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.colored_label(state_color, state_txt);
                    ui.label(RichText::new(fmt_hms(elapsed)).monospace().size(56.0).strong());
                    ui.label(format!("{} {}", s.started_label, lang.dt(&t.started_at)));
                });

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if t.paused {
                        if ui.button(s.resume).clicked() {
                            act = Act::Resume;
                        }
                    } else if ui.button(s.pause).clicked() {
                        act = Act::Pause;
                    }
                    if ui.button(s.finish_save).clicked() {
                        act = Act::Finish;
                    }
                    ui.separator();
                    if self.cancel_armed {
                        ui.colored_label(Color32::from_rgb(230, 110, 110), s.discard_running_q);
                        if ui.button(s.yes_discard).clicked() {
                            act = Act::CancelDo;
                        }
                        if ui.button(s.no_keep).clicked() {
                            act = Act::CancelUnarm;
                        }
                    } else if ui.button(s.cancel).clicked() {
                        act = Act::CancelArm;
                    }
                });

                // Pending attachments
                ui.add_space(12.0);
                ui.separator();
                ui.label(RichText::new(s.session_attachments).strong());
                if self.pending_attachments.is_empty() {
                    ui.label(s.no_attachments_yet);
                } else {
                    let mut remove: Option<usize> = None;
                    for (i, p) in self.pending_attachments.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(p.display().to_string());
                            if ui.small_button(s.remove).clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        self.pending_attachments.remove(i);
                    }
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.picker_busy, egui::Button::new(s.add_files))
                        .clicked()
                        {
                            want_picker = true;
                        }
                        let edit = egui::TextEdit::singleline(&mut self.manual_path)
                        .hint_text(s.path_hint)
                        .desired_width(320.0);
                    ui.add(edit);
                    if ui.button(s.add).clicked() {
                        self.status = add_manual_path(
                            &mut self.pending_attachments,
                            &mut self.manual_path,
                            lang,
                        );
                    }
                });
            }
        }

        if want_picker {
            let ctx = ui.ctx().clone();
            self.open_file_picker(ctx);
        }

        match act {
            Act::None => {}
            Act::Start => {
                self.timer = Some(RunningTimer::new());
                self.pending_attachments.clear();
                self.cancel_armed = false;
                self.status = s.timer_started.to_string();
            }
            Act::Pause => {
                if let Some(t) = self.timer.as_mut() {
                    t.pause();
                }
                self.status = s.timer_paused_msg.to_string();
            }
            Act::Resume => {
                if let Some(t) = self.timer.as_mut() {
                    t.resume();
                }
                self.status = s.resumed.to_string();
            }
            Act::Finish => {
                if let Some(t) = self.timer.take() {
                    self.draft = Some(Draft {
                        started_at: t.started_at,
                        ended_at: Utc::now(),
                                      duration_secs: t.elapsed_secs().max(0) as u64,
                                      topic: String::new(),
                                      notes: String::new(),
                                      tags: String::new(),
                                      attachments: std::mem::take(&mut self.pending_attachments),
                                      discard_armed: false,
                    });
                    self.cancel_armed = false;
                    self.status = s.finish_fill.to_string();
                }
            }
            Act::CancelArm => self.cancel_armed = true,
            Act::CancelUnarm => self.cancel_armed = false,
            Act::CancelDo => {
                self.timer = None;
                self.pending_attachments.clear();
                self.cancel_armed = false;
                self.status = s.discarded_msg.to_string();
            }
        }
    }

    fn ui_draft_form(&mut self, ui: &mut egui::Ui) {
        let lang = self.db.lang;
        let s = lang.s();

        let mut do_save = false;
        let mut do_discard = false;
        let mut want_picker = false;

        if let Some(d) = self.draft.as_mut() {
            ui.add_space(8.0);
            ui.heading(s.save_session);
            ui.add_space(8.0);

            egui::Grid::new("draft_grid")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                ui.label(s.duration_label);
                ui.label(RichText::new(lang.dur_human(d.duration_secs as i64)).strong());
                ui.end_row();
                ui.label(s.started_label);
                ui.label(lang.dt(&d.started_at));
                ui.end_row();
                ui.label(s.ended_label);
                ui.label(lang.dt(&d.ended_at));
                ui.end_row();
            });

            ui.add_space(10.0);
            ui.label(s.topic_label);
            ui.add(
                egui::TextEdit::singleline(&mut d.topic)
                .hint_text(s.topic_hint)
                .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);
            ui.label(s.notes_label);
            ui.add(
                egui::TextEdit::multiline(&mut d.notes)
                .hint_text(s.notes_hint)
                .desired_rows(5)
                .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);
            ui.label(s.tags_label);
            ui.add(
                egui::TextEdit::singleline(&mut d.tags)
                .hint_text(s.tags_hint)
                .desired_width(f32::INFINITY),
            );

            ui.add_space(10.0);
            ui.label(RichText::new(s.attachments_label).strong());
            if d.attachments.is_empty() {
                ui.label(s.no_attachments_yet);
            } else {
                let mut remove: Option<usize> = None;
                for (i, p) in d.attachments.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(p.display().to_string());
                        if ui.small_button(s.remove).clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    d.attachments.remove(i);
                }
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.picker_busy, egui::Button::new(s.add_files))
                    .clicked()
                    {
                        want_picker = true;
                    }
                    let edit = egui::TextEdit::singleline(&mut self.manual_path)
                    .hint_text(s.path_hint)
                    .desired_width(320.0);
                ui.add(edit);
                if ui.button(s.add).clicked() {
                    self.status = add_manual_path(&mut d.attachments, &mut self.manual_path, lang);
                }
            });

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(RichText::new(s.save).size(16.0))
                .min_size(egui::vec2(140.0, 36.0));
                if ui.add(save_btn).clicked() {
                    do_save = true;
                }
                ui.separator();
                if d.discard_armed {
                    ui.colored_label(Color32::from_rgb(230, 110, 110), s.sure_q);
                    if ui.button(s.yes_dont_save).clicked() {
                        do_discard = true;
                    }
                    if ui.button(s.no_keep).clicked() {
                        d.discard_armed = false;
                    }
                } else if ui.button(s.discard).clicked() {
                    d.discard_armed = true;
                }
            });
        }

        if want_picker {
            let ctx = ui.ctx().clone();
            self.open_file_picker(ctx);
        }

        if do_save {
            if let Some(d) = self.draft.take() {
                self.persist_draft(d);
            }
        } else if do_discard {
            self.draft = None;
            self.status = s.discarded_msg.to_string();
        }
    }

    fn ui_sessions(&mut self, ui: &mut egui::Ui) {
        let lang = self.db.lang;
        let s = lang.s();

        if self.db.sessions.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(s.no_sessions_1);
                ui.label(s.no_sessions_2);
            });
            return;
        }

        let total: i64 = self.db.sessions.iter().map(|x| x.duration_secs as i64).sum();
        ui.add_space(4.0);
        ui.label(lang.sessions_summary(self.db.sessions.len(), &lang.dur_human(total)));
        ui.add_space(4.0);
        ui.separator();

        let current_confirm = self.confirm_delete;
        let mut set_confirm: Option<u64> = None;
        let mut clear_confirm = false;
        let mut delete_id: Option<u64> = None;
        let mut open_req: Option<PathBuf> = None;

        egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for sess in self.db.sessions.iter().rev() {
                let topic = if sess.topic.is_empty() {
                    s.no_topic
                } else {
                    sess.topic.as_str()
                };
                let title = format!(
                    "{}  •  {}  •  {}",
                    lang.dt(&sess.started_at),
                                    lang.dur_human(sess.duration_secs as i64),
                                    topic
                );

                egui::CollapsingHeader::new(title)
                .id_salt(sess.id)
                .show(ui, |ui| {
                    egui::Grid::new(("session_grid", sess.id))
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(s.ended_label);
                        ui.label(lang.dt(&sess.ended_at));
                        ui.end_row();
                        if !sess.tags.is_empty() {
                            ui.label(s.tags_colon);
                            ui.label(sess.tags.join(", "));
                            ui.end_row();
                        }
                    });

                    if !sess.notes.is_empty() {
                        ui.add_space(4.0);
                        ui.label(RichText::new(s.notes_colon).strong());
                        ui.label(&sess.notes);
                    }

                    if !sess.attachments.is_empty() {
                        ui.add_space(4.0);
                        ui.label(RichText::new(s.attachments_colon).strong());
                        for a in &sess.attachments {
                            ui.horizontal(|ui| {
                                ui.label(&a.file_name);
                                if ui.small_button(s.open).clicked() {
                                    open_req = Some(a.path.clone());
                                }
                            });
                        }
                        if ui.small_button(s.open_attach_dir).clicked() {
                            open_req =
                            Some(self.paths.attach_dir.join(sess.id.to_string()));
                        }
                    }

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if current_confirm == Some(sess.id) {
                            ui.colored_label(
                                Color32::from_rgb(230, 110, 110),
                                             s.delete_q,
                            );
                            if ui.button(s.yes_delete).clicked() {
                                delete_id = Some(sess.id);
                            }
                            if ui.button(s.no_keep).clicked() {
                                clear_confirm = true;
                            }
                        } else if ui.button(s.delete).clicked() {
                            set_confirm = Some(sess.id);
                        }
                    });
                });
            }
        });

        if let Some(p) = open_req {
            open_in_system(&p);
        }
        if let Some(id) = set_confirm {
            self.confirm_delete = Some(id);
        }
        if clear_confirm {
            self.confirm_delete = None;
        }
        if let Some(id) = delete_id {
            let _ = fs::remove_dir_all(self.paths.attach_dir.join(id.to_string()));
            self.db.sessions.retain(|x| x.id != id);
            self.confirm_delete = None;
            self.save();
            self.status = s.deleted_msg.to_string();
        }
    }

    fn ui_stats(&mut self, ui: &mut egui::Ui) {
        let lang = self.db.lang;
        let s = lang.s();

        if self.db.sessions.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(s.stats_empty);
            });
            return;
        }

        let total: i64 = self.db.sessions.iter().map(|x| x.duration_secs as i64).sum();
        let count = self.db.sessions.len() as i64;

        // Totals per day and per topic
        let mut by_day: BTreeMap<NaiveDate, i64> = BTreeMap::new();
        let mut by_topic: BTreeMap<String, i64> = BTreeMap::new();
        for sess in &self.db.sessions {
            let day = sess.started_at.with_timezone(&Local).date_naive();
            *by_day.entry(day).or_default() += sess.duration_secs as i64;
            let key = if sess.topic.trim().is_empty() {
                s.no_topic.to_string()
            } else {
                sess.topic.trim().to_string()
            };
            *by_topic.entry(key).or_default() += sess.duration_secs as i64;
        }

        // Streak: unbroken days counting back from today (or yesterday if
        // today is empty).
        let today = Local::now().date_naive();
        let mut streak = 0u32;
        let mut cursor = if by_day.contains_key(&today) {
            Some(today)
        } else {
            today.pred_opt()
        };
        while let Some(d) = cursor {
            if by_day.contains_key(&d) {
                streak += 1;
                cursor = d.pred_opt();
            } else {
                break;
            }
        }

        ui.add_space(8.0);
        egui::Grid::new("stats_grid")
        .num_columns(2)
        .spacing([24.0, 6.0])
        .show(ui, |ui| {
            ui.label(s.total_time);
            ui.label(RichText::new(lang.dur_human(total)).strong());
            ui.end_row();
            ui.label(s.session_count);
            ui.label(RichText::new(count.to_string()).strong());
            ui.end_row();
            ui.label(s.avg_session);
            ui.label(RichText::new(lang.dur_human(total / count.max(1))).strong());
            ui.end_row();
            ui.label(s.streak_label);
            ui.label(RichText::new(lang.streak_line(streak)).strong());
            ui.end_row();
            ui.label(s.active_days);
            ui.label(RichText::new(format!("{}", by_day.len())).strong());
            ui.end_row();
        });

        ui.add_space(12.0);
        ui.separator();
        ui.label(RichText::new(s.last_14).strong());
        ui.add_space(4.0);

        let mut last14: Vec<(NaiveDate, i64)> = Vec::new();
        for off in (0..14u64).rev() {
            let d = today.checked_sub_days(Days::new(off)).unwrap_or(today);
            last14.push((d, by_day.get(&d).copied().unwrap_or(0)));
        }
        let max_day = last14.iter().map(|(_, v)| *v).max().unwrap_or(0).max(1);

        egui::Grid::new("days_grid")
        .num_columns(3)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for (d, secs) in &last14 {
                ui.label(lang.date_short(*d));
                ui.add(
                    egui::ProgressBar::new(*secs as f32 / max_day as f32)
                    .desired_width(320.0),
                );
                if *secs > 0 {
                    ui.label(lang.dur_human(*secs));
                } else {
                    ui.label("—");
                }
                ui.end_row();
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.label(RichText::new(s.by_topic).strong());
        ui.add_space(4.0);

        let mut topics: Vec<(String, i64)> = by_topic.into_iter().collect();
        topics.sort_by(|a, b| b.1.cmp(&a.1));

        egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("topics_grid")
            .num_columns(2)
            .spacing([24.0, 4.0])
            .show(ui, |ui| {
                for (topic, secs) in topics.iter().take(15) {
                    ui.label(topic);
                    ui.label(lang.dur_human(*secs));
                    ui.end_row();
                }
            });
        });
    }

    fn ui_export(&mut self, ui: &mut egui::Ui) {
        let lang = self.db.lang;
        let s = lang.s();

        ui.add_space(8.0);
        ui.heading(s.export_heading);
        ui.add_space(6.0);
        ui.label(s.export_csv_info);
        ui.label(s.export_json_info);
        ui.add_space(10.0);

        let export_dir = dirs::document_dir()
        .filter(|p| p.exists())
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));

        ui.horizontal(|ui| {
            if ui.button(s.export_csv_btn).clicked() {
                match export_csv(&self.db, &export_dir) {
                    Ok(p) => {
                        self.status = format!("{} {}", s.csv_written, p.display());
                        self.last_export = Some(p);
                    }
                    Err(e) => self.status = format!("{} {e}", s.export_err),
                }
            }
            if ui.button(s.export_json_btn).clicked() {
                match export_json(&self.db, &export_dir) {
                    Ok(p) => {
                        self.status = format!("{} {}", s.json_written, p.display());
                        self.last_export = Some(p);
                    }
                    Err(e) => self.status = format!("{} {e}", s.export_err),
                }
            }
        });

        if let Some(p) = self.last_export.clone() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(format!("{} {}", s.last_export_label, p.display()));
                if ui.small_button(s.open_folder).clicked() {
                    open_in_system(p.parent().unwrap_or(&p));
                }
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(format!("{} {}", s.data_dir_label, self.paths.data_dir.display()));
            if ui.small_button(s.open).clicked() {
                let _ = fs::create_dir_all(&self.paths.data_dir);
                open_in_system(&self.paths.data_dir);
            }
        });
        ui.label(s.data_dir_info);
    }
}

impl eframe::App for MesaiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Results coming back from the native file picker
        while let Ok(paths) = self.picker_rx.try_recv() {
            self.picker_busy = false;
            if !paths.is_empty() {
                self.route_new_attachments(paths);
            }
        }

        // Files dropped onto the window (only delivered on X11; winit's
        // Wayland backend has no drag-and-drop support — use "Add files…")
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
            .dropped_files
            .iter()
            .filter_map(|f| f.path.clone())
            .collect()
        });
        if !dropped.is_empty() {
            self.route_new_attachments(dropped);
        }

        // Keep repainting while the timer is ticking
        if self.timer.as_ref().map(|t| !t.paused).unwrap_or(false) {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }

        let lang = self.db.lang;
        let s = lang.s();

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Mesai").heading().strong());
                ui.separator();
                ui.selectable_value(&mut self.tab, Tab::Timer, s.tab_timer);
                ui.selectable_value(&mut self.tab, Tab::Sessions, s.tab_sessions);
                ui.selectable_value(&mut self.tab, Tab::Stats, s.tab_stats);
                ui.selectable_value(&mut self.tab, Tab::Export, s.tab_export);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Language selector (persisted in data.json)
                    let mut lang_sel = self.db.lang;
                    egui::ComboBox::from_id_salt("lang_select")
                    .selected_text(match lang_sel {
                        Lang::En => "EN",
                        Lang::Tr => "TR",
                    })
                    .width(64.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut lang_sel, Lang::En, "English");
                        ui.selectable_value(&mut lang_sel, Lang::Tr, "Türkçe");
                    });
                    if lang_sel != self.db.lang {
                        self.db.lang = lang_sel;
                        self.status = self.db.lang.s().ready.to_string();
                        self.save();
                    }

                    if let Some(t) = &self.timer {
                        let (txt, color) = if t.paused {
                            (s.paused, Color32::from_rgb(230, 190, 90))
                        } else {
                            (s.running, Color32::from_rgb(130, 200, 130))
                        };
                        ui.colored_label(color, format!("{txt} · {}", fmt_hms(t.elapsed_secs())));
                    }
                });
            });
            ui.add_space(6.0);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.label(&self.status);
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Timer => self.ui_timer(ui),
                                           Tab::Sessions => self.ui_sessions(ui),
                                           Tab::Stats => self.ui_stats(ui),
                                           Tab::Export => self.ui_export(ui),
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let viewport = egui::ViewportBuilder::default()
    .with_app_id(APP_ID) // Wayland app_id: shows up as "mesai" in window rules
    .with_inner_size([960.0, 660.0])
    .with_min_inner_size([720.0, 480.0]);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Mesai",
        options,
        Box::new(|cc| Ok(Box::new(MesaiApp::new(cc)))),
    )
}

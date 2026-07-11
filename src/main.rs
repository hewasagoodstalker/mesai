// mesai — basit çalışma takip uygulaması
//
// Ne yapar?
//   * Sayaç: çalışmaya başlarken başlat, istersen duraklat, bitince kaydet.
//   * Oturum kaydı: konu, not, etiketler ve ek dosyalar (ör. yazdığın .c dosyaları).
//   * Ekler: dosyayı pencereye sürükleyip bırak ya da yolunu elle yaz.
//   * İstatistik: toplam süre, gün serisi, son 14 gün, konu dağılımı.
//   * Dışa aktarma: CSV (tablo programları için) ve JSON (tam yedek).
//
// Veriler ~/.local/share/mesai/ altında tutulur:
//   data.json          -> tüm oturumlar
//   attachments/<id>/  -> o oturuma eklenen dosyaların kopyaları
//
// Arayüz eframe/egui ile yazıldı: winit üzerinden Wayland'ı doğrudan konuşur
// (WAYLAND_DISPLAY tanımlıysa Wayland, değilse X11'e düşer). Pencerenin
// Wayland app_id'si "mesai"dir; pencere kurallarında bunu kullanabilirsin.

use chrono::{DateTime, Datelike, Days, Local, NaiveDate, Utc};
use eframe::egui;
use egui::{Color32, RichText};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const APP_ID: &str = "mesai";

// ---------------------------------------------------------------------------
// Veri modeli
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
struct Attachment {
    file_name: String,
    path: PathBuf, // attachments/<id>/ altındaki kopyanın yolu
}

#[derive(Serialize, Deserialize, Clone)]
struct Session {
    id: u64,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    /// Aktif çalışma süresi (duraklatmalar hariç), saniye.
    duration_secs: u64,
    topic: String,
    notes: String,
    tags: Vec<String>,
    attachments: Vec<Attachment>,
}

#[derive(Serialize, Deserialize, Default)]
struct Database {
    next_id: u64,
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

/// Önce geçici dosyaya yazıp sonra yeniden adlandırır; yazma yarıda kesilirse
/// eski veri bozulmadan kalır.
fn save_db(paths: &Paths, db: &Database) -> std::io::Result<()> {
    fs::create_dir_all(&paths.data_dir)?;
    let tmp = paths.db_file.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(db).expect("veri JSON'a çevrilemedi");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &paths.db_file)
}

/// Kaynak dosyayı attachments/<id>/ altına kopyalar. Aynı adda dosya varsa
/// sonuna _1, _2 ... ekler.
fn copy_attachment(attach_dir: &Path, id: u64, src: &Path) -> std::io::Result<Attachment> {
    let dir = attach_dir.join(id.to_string());
    fs::create_dir_all(&dir)?;

    let name = src
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ek".to_string());

    let mut dest = dir.join(&name);
    let mut n = 1;
    while dest.exists() {
        let stem = Path::new(&name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("ek");
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
// Küçük yardımcılar
// ---------------------------------------------------------------------------

const AYLAR: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran",
    "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık",
];
const GUNLER: [&str; 7] = ["Pzt", "Sal", "Çar", "Per", "Cum", "Cts", "Paz"];

fn fmt_hms(total: i64) -> String {
    let t = total.max(0);
    format!("{:02}:{:02}:{:02}", t / 3600, (t % 3600) / 60, t % 60)
}

fn fmt_dur_human(total: i64) -> String {
    let t = total.max(0);
    let (h, m, s) = (t / 3600, (t % 3600) / 60, t % 60);
    if h > 0 {
        format!("{h} sa {m} dk")
    } else if m > 0 {
        format!("{m} dk {s} sn")
    } else {
        format!("{s} sn")
    }
}

/// "11 Temmuz 2026, 14:30" (yerel saat)
fn fmt_dt(dt: &DateTime<Utc>) -> String {
    let l = dt.with_timezone(&Local);
    format!(
        "{} {} {}, {}",
        l.day(),
        AYLAR[l.month0() as usize],
        l.year(),
        l.format("%H:%M")
    )
}

/// "Cts 11 Tem" gibi kısa gün etiketi
fn fmt_date_short(d: NaiveDate) -> String {
    let ay = &AYLAR[d.month0() as usize][..3.min(AYLAR[d.month0() as usize].len())];
    format!("{} {} {}", GUNLER[d.weekday().num_days_from_monday() as usize], d.day(), ay)
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

/// Elle yazılan yolu doğrulayıp listeye ekler; kullanıcıya gösterilecek mesajı döndürür.
fn add_manual_path(list: &mut Vec<PathBuf>, input: &mut String) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "Önce bir dosya yolu yaz.".to_string();
    }
    let p = expand_tilde(trimmed);
    if p.is_file() {
        let msg = format!("Eklendi: {}", p.display());
        list.push(p);
        input.clear();
        msg
    } else {
        format!("Dosya bulunamadı: {}", p.display())
    }
}

/// Dosya ya da klasörü sistemin varsayılan uygulamasıyla açar.
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
        String::from("id,baslangic,bitis,sure_saniye,sure,konu,etiketler,not,ek_sayisi\n");
    for s in &db.sessions {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            s.id,
            s.started_at.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
            s.ended_at.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
            s.duration_secs,
            csv_field(&fmt_dur_human(s.duration_secs as i64)),
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
// Sayaç ve taslak (kaydedilmeyi bekleyen oturum)
// ---------------------------------------------------------------------------

/// Süre, Instant yerine duvar saatiyle ölçülür; böylece dizüstü uykuya
/// girip uyansa bile geçen süre doğru kalır.
struct RunningTimer {
    started_at: DateTime<Utc>,
    seg_start: DateTime<Utc>, // son "devam" anı
    accumulated_secs: i64,    // önceki segmentlerin toplamı
    paused: bool,
}

impl RunningTimer {
    fn new() -> Self {
        let now = Utc::now();
        Self { started_at: now, seg_start: now, accumulated_secs: 0, paused: false }
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

/// Sayaç bittikten sonra doldurulan form.
struct Draft {
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    duration_secs: u64,
    topic: String,
    notes: String,
    tags: String,
    attachments: Vec<PathBuf>,
    discard_armed: bool, // "vazgeç" için iki tıklamalı onay
}

// ---------------------------------------------------------------------------
// Uygulama
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
    pending_attachments: Vec<PathBuf>, // sayaç çalışırken biriken ekler
    draft: Option<Draft>,

    manual_path: String,        // elle yol girme kutusu
    status: String,             // alttaki durum satırı
    cancel_armed: bool,         // çalışan sayaç için iki tıklamalı iptal
    confirm_delete: Option<u64>,
    last_export: Option<PathBuf>,
}

impl MesaiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let paths = app_paths();
        let db = load_db(&paths);
        Self {
            paths,
            db,
            tab: Tab::Timer,
            timer: None,
            pending_attachments: Vec::new(),
            draft: None,
            manual_path: String::new(),
            status: "Hazır.".to_string(),
            cancel_armed: false,
            confirm_delete: None,
            last_export: None,
        }
    }

    fn save(&mut self) {
        if let Err(e) = save_db(&self.paths, &self.db) {
            self.status = format!("Kaydetme hatası: {e}");
        }
    }

    /// Taslağı kalıcı oturuma çevirir: ekleri kopyalar, veritabanına yazar.
    fn persist_draft(&mut self, d: Draft) {
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
            "Oturum kaydedildi.".to_string()
        } else {
            format!("Oturum kaydedildi ({failed} ek kopyalanamadı).")
        };
    }

    // -- Sekmeler ------------------------------------------------------------

    fn ui_timer(&mut self, ui: &mut egui::Ui) {
        if self.draft.is_some() {
            self.ui_draft_form(ui);
            return;
        }

        #[derive(PartialEq)]
        enum Act { None, Start, Pause, Resume, Finish, CancelArm, CancelUnarm, CancelDo }
        let mut act = Act::None;

        match &self.timer {
            None => {
                ui.add_space(36.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("00:00:00").monospace().size(56.0).weak());
                    ui.add_space(16.0);
                    let btn = egui::Button::new(RichText::new("Başlat").size(18.0))
                        .min_size(egui::vec2(180.0, 44.0));
                    if ui.add(btn).clicked() {
                        act = Act::Start;
                    }
                    ui.add_space(20.0);
                    ui.label(
                        "İpucu: Sayaç çalışırken dosyaları pencereye sürükleyip bırakırsan\n\
                         oturuma ek olarak kaydedilir.",
                    );
                });
            }
            Some(t) => {
                let elapsed = t.elapsed_secs();
                let (state_txt, state_color) = if t.paused {
                    ("Duraklatıldı", Color32::from_rgb(230, 190, 90))
                } else {
                    ("Çalışıyor", Color32::from_rgb(130, 200, 130))
                };

                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.colored_label(state_color, state_txt);
                    ui.label(RichText::new(fmt_hms(elapsed)).monospace().size(56.0).strong());
                    ui.label(format!("Başlangıç: {}", fmt_dt(&t.started_at)));
                });

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if t.paused {
                        if ui.button("Devam et").clicked() {
                            act = Act::Resume;
                        }
                    } else if ui.button("Duraklat").clicked() {
                        act = Act::Pause;
                    }
                    if ui.button("Bitir ve kaydet").clicked() {
                        act = Act::Finish;
                    }
                    ui.separator();
                    if self.cancel_armed {
                        ui.colored_label(Color32::from_rgb(230, 110, 110), "Kaydedilmeden silinsin mi?");
                        if ui.button("Evet, sil").clicked() {
                            act = Act::CancelDo;
                        }
                        if ui.button("Vazgeç").clicked() {
                            act = Act::CancelUnarm;
                        }
                    } else if ui.button("İptal").clicked() {
                        act = Act::CancelArm;
                    }
                });

                // Bekleyen ekler
                ui.add_space(12.0);
                ui.separator();
                ui.label(RichText::new("Bu oturumun ekleri").strong());
                if self.pending_attachments.is_empty() {
                    ui.label("Henüz ek yok. Dosyayı pencereye bırak ya da aşağıya yolunu yaz.");
                } else {
                    let mut remove: Option<usize> = None;
                    for (i, p) in self.pending_attachments.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(p.display().to_string());
                            if ui.small_button("Kaldır").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        self.pending_attachments.remove(i);
                    }
                }
                ui.horizontal(|ui| {
                    let edit = egui::TextEdit::singleline(&mut self.manual_path)
                        .hint_text("Dosya yolu (ör. ~/proje/main.c)")
                        .desired_width(360.0);
                    ui.add(edit);
                    if ui.button("Ekle").clicked() {
                        self.status =
                            add_manual_path(&mut self.pending_attachments, &mut self.manual_path);
                    }
                });
            }
        }

        match act {
            Act::None => {}
            Act::Start => {
                self.timer = Some(RunningTimer::new());
                self.pending_attachments.clear();
                self.cancel_armed = false;
                self.status = "Sayaç başladı. Kolay gelsin!".to_string();
            }
            Act::Pause => {
                if let Some(t) = self.timer.as_mut() {
                    t.pause();
                }
                self.status = "Sayaç duraklatıldı.".to_string();
            }
            Act::Resume => {
                if let Some(t) = self.timer.as_mut() {
                    t.resume();
                }
                self.status = "Devam ediliyor.".to_string();
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
                    self.status = "Oturum bitti; bilgileri doldurup kaydet.".to_string();
                }
            }
            Act::CancelArm => self.cancel_armed = true,
            Act::CancelUnarm => self.cancel_armed = false,
            Act::CancelDo => {
                self.timer = None;
                self.pending_attachments.clear();
                self.cancel_armed = false;
                self.status = "Oturum kaydedilmeden iptal edildi.".to_string();
            }
        }
    }

    fn ui_draft_form(&mut self, ui: &mut egui::Ui) {
        let mut do_save = false;
        let mut do_discard = false;

        if let Some(d) = self.draft.as_mut() {
            ui.add_space(8.0);
            ui.heading("Oturumu kaydet");
            ui.add_space(8.0);

            egui::Grid::new("draft_grid")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Süre:");
                    ui.label(
                        RichText::new(fmt_dur_human(d.duration_secs as i64)).strong(),
                    );
                    ui.end_row();
                    ui.label("Başlangıç:");
                    ui.label(fmt_dt(&d.started_at));
                    ui.end_row();
                    ui.label("Bitiş:");
                    ui.label(fmt_dt(&d.ended_at));
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.label("Konu");
            ui.add(
                egui::TextEdit::singleline(&mut d.topic)
                    .hint_text("ör. C — Milestone 2: strtok ile tokenizasyon")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);
            ui.label("Notlar");
            ui.add(
                egui::TextEdit::multiline(&mut d.notes)
                    .hint_text("Bugün ne yaptın, nerede takıldın, yarın nereden devam edeceksin?")
                    .desired_rows(5)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);
            ui.label("Etiketler (virgülle ayır)");
            ui.add(
                egui::TextEdit::singleline(&mut d.tags)
                    .hint_text("ör. c, shell, pointer")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(10.0);
            ui.label(RichText::new("Ekler").strong());
            if d.attachments.is_empty() {
                ui.label("Ek yok. Dosyayı pencereye bırakabilir ya da yolunu yazabilirsin.");
            } else {
                let mut remove: Option<usize> = None;
                for (i, p) in d.attachments.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(p.display().to_string());
                        if ui.small_button("Kaldır").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    d.attachments.remove(i);
                }
            }
            ui.horizontal(|ui| {
                let edit = egui::TextEdit::singleline(&mut self.manual_path)
                    .hint_text("Dosya yolu (ör. ~/proje/main.c)")
                    .desired_width(360.0);
                ui.add(edit);
                if ui.button("Ekle").clicked() {
                    self.status = add_manual_path(&mut d.attachments, &mut self.manual_path);
                }
            });

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(RichText::new("Kaydet").size(16.0))
                    .min_size(egui::vec2(140.0, 36.0));
                if ui.add(save_btn).clicked() {
                    do_save = true;
                }
                ui.separator();
                if d.discard_armed {
                    ui.colored_label(Color32::from_rgb(230, 110, 110), "Emin misin?");
                    if ui.button("Evet, kaydetme").clicked() {
                        do_discard = true;
                    }
                    if ui.button("Vazgeç").clicked() {
                        d.discard_armed = false;
                    }
                } else if ui.button("Kaydetmeden sil").clicked() {
                    d.discard_armed = true;
                }
            });
        }

        if do_save {
            if let Some(d) = self.draft.take() {
                self.persist_draft(d);
            }
        } else if do_discard {
            self.draft = None;
            self.status = "Oturum kaydedilmedi.".to_string();
        }
    }

    fn ui_sessions(&mut self, ui: &mut egui::Ui) {
        if self.db.sessions.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label("Henüz kayıt yok.");
                ui.label("Sayaç sekmesinden ilk çalışmanı başlat!");
            });
            return;
        }

        let total: i64 = self.db.sessions.iter().map(|s| s.duration_secs as i64).sum();
        ui.add_space(4.0);
        ui.label(format!(
            "Toplam {} oturum • {}",
            self.db.sessions.len(),
            fmt_dur_human(total)
        ));
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
                for s in self.db.sessions.iter().rev() {
                    let topic = if s.topic.is_empty() { "(konu yok)" } else { s.topic.as_str() };
                    let title = format!(
                        "{}  •  {}  •  {}",
                        fmt_dt(&s.started_at),
                        fmt_dur_human(s.duration_secs as i64),
                        topic
                    );

                    egui::CollapsingHeader::new(title)
                        .id_salt(s.id)
                        .show(ui, |ui| {
                            egui::Grid::new(("session_grid", s.id))
                                .num_columns(2)
                                .spacing([16.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("Bitiş:");
                                    ui.label(fmt_dt(&s.ended_at));
                                    ui.end_row();
                                    if !s.tags.is_empty() {
                                        ui.label("Etiketler:");
                                        ui.label(s.tags.join(", "));
                                        ui.end_row();
                                    }
                                });

                            if !s.notes.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new("Not:").strong());
                                ui.label(&s.notes);
                            }

                            if !s.attachments.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new("Ekler:").strong());
                                for a in &s.attachments {
                                    ui.horizontal(|ui| {
                                        ui.label(&a.file_name);
                                        if ui.small_button("Aç").clicked() {
                                            open_req = Some(a.path.clone());
                                        }
                                    });
                                }
                                if ui.small_button("Ek klasörünü aç").clicked() {
                                    open_req =
                                        Some(self.paths.attach_dir.join(s.id.to_string()));
                                }
                            }

                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if current_confirm == Some(s.id) {
                                    ui.colored_label(
                                        Color32::from_rgb(230, 110, 110),
                                        "Kayıt ve ekleri silinsin mi?",
                                    );
                                    if ui.button("Evet, sil").clicked() {
                                        delete_id = Some(s.id);
                                    }
                                    if ui.button("Vazgeç").clicked() {
                                        clear_confirm = true;
                                    }
                                } else if ui.button("Sil").clicked() {
                                    set_confirm = Some(s.id);
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
            self.db.sessions.retain(|s| s.id != id);
            self.confirm_delete = None;
            self.save();
            self.status = "Kayıt silindi.".to_string();
        }
    }

    fn ui_stats(&mut self, ui: &mut egui::Ui) {
        if self.db.sessions.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label("İstatistik için önce birkaç oturum kaydetmen gerekiyor.");
            });
            return;
        }

        let total: i64 = self.db.sessions.iter().map(|s| s.duration_secs as i64).sum();
        let count = self.db.sessions.len() as i64;

        // Günlere ve konulara göre toplamlar
        let mut by_day: BTreeMap<NaiveDate, i64> = BTreeMap::new();
        let mut by_topic: BTreeMap<String, i64> = BTreeMap::new();
        for s in &self.db.sessions {
            let day = s.started_at.with_timezone(&Local).date_naive();
            *by_day.entry(day).or_default() += s.duration_secs as i64;
            let key = if s.topic.trim().is_empty() {
                "(konu yok)".to_string()
            } else {
                s.topic.trim().to_string()
            };
            *by_topic.entry(key).or_default() += s.duration_secs as i64;
        }

        // Seri: bugünden (bugün boşsa dünden) geriye kesintisiz gün sayısı
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
                ui.label("Toplam süre:");
                ui.label(RichText::new(fmt_dur_human(total)).strong());
                ui.end_row();
                ui.label("Oturum sayısı:");
                ui.label(RichText::new(count.to_string()).strong());
                ui.end_row();
                ui.label("Ortalama oturum:");
                ui.label(RichText::new(fmt_dur_human(total / count.max(1))).strong());
                ui.end_row();
                ui.label("Seri:");
                ui.label(RichText::new(format!("{streak} gün üst üste")).strong());
                ui.end_row();
                ui.label("Farklı gün:");
                ui.label(RichText::new(format!("{}", by_day.len())).strong());
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();
        ui.label(RichText::new("Son 14 gün").strong());
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
                    ui.label(fmt_date_short(*d));
                    ui.add(
                        egui::ProgressBar::new(*secs as f32 / max_day as f32)
                            .desired_width(320.0),
                    );
                    if *secs > 0 {
                        ui.label(fmt_dur_human(*secs));
                    } else {
                        ui.label("—");
                    }
                    ui.end_row();
                }
            });

        ui.add_space(12.0);
        ui.separator();
        ui.label(RichText::new("Konulara göre").strong());
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
                            ui.label(fmt_dur_human(*secs));
                            ui.end_row();
                        }
                    });
            });
    }

    fn ui_export(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.heading("Dışa aktar");
        ui.add_space(6.0);
        ui.label("CSV: tablo programlarında (LibreOffice Calc vb.) açmak için özet tablo.");
        ui.label("JSON: tüm verinin tam yedeği; ek dosyaların yolları da içinde.");
        ui.add_space(10.0);

        let export_dir = dirs::document_dir()
            .filter(|p| p.exists())
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));

        ui.horizontal(|ui| {
            if ui.button("CSV olarak dışa aktar").clicked() {
                match export_csv(&self.db, &export_dir) {
                    Ok(p) => {
                        self.status = format!("CSV yazıldı: {}", p.display());
                        self.last_export = Some(p);
                    }
                    Err(e) => self.status = format!("CSV hatası: {e}"),
                }
            }
            if ui.button("JSON olarak dışa aktar").clicked() {
                match export_json(&self.db, &export_dir) {
                    Ok(p) => {
                        self.status = format!("JSON yazıldı: {}", p.display());
                        self.last_export = Some(p);
                    }
                    Err(e) => self.status = format!("JSON hatası: {e}"),
                }
            }
        });

        if let Some(p) = self.last_export.clone() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(format!("Son dışa aktarma: {}", p.display()));
                if ui.small_button("Klasörü aç").clicked() {
                    open_in_system(p.parent().unwrap_or(&p));
                }
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(format!("Veri klasörü: {}", self.paths.data_dir.display()));
            if ui.small_button("Aç").clicked() {
                let _ = fs::create_dir_all(&self.paths.data_dir);
                open_in_system(&self.paths.data_dir);
            }
        });
        ui.label("Tüm oturumlar data.json içinde; ekler attachments/ altında saklanır.");
    }
}

impl eframe::App for MesaiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Pencereye bırakılan dosyaları topla
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !dropped.is_empty() {
            let n = dropped.len();
            if let Some(d) = self.draft.as_mut() {
                d.attachments.extend(dropped);
                self.status = format!("{n} ek eklendi.");
            } else if self.timer.is_some() {
                self.pending_attachments.extend(dropped);
                self.status = format!("{n} ek eklendi.");
            } else {
                self.status =
                    "Ek kaydetmek için önce sayacı başlat (ekler oturuma bağlanır).".to_string();
            }
        }

        // Sayaç akarken ekranı tazele
        if self.timer.as_ref().map(|t| !t.paused).unwrap_or(false) {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Mesai").heading().strong());
                ui.separator();
                ui.selectable_value(&mut self.tab, Tab::Timer, "Sayaç");
                ui.selectable_value(&mut self.tab, Tab::Sessions, "Kayıtlar");
                ui.selectable_value(&mut self.tab, Tab::Stats, "İstatistik");
                ui.selectable_value(&mut self.tab, Tab::Export, "Dışa Aktar");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(t) = &self.timer {
                        let (txt, color) = if t.paused {
                            ("Duraklatıldı", Color32::from_rgb(230, 190, 90))
                        } else {
                            ("Çalışıyor", Color32::from_rgb(130, 200, 130))
                        };
                        ui.colored_label(
                            color,
                            format!("{txt} · {}", fmt_hms(t.elapsed_secs())),
                        );
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
        .with_app_id(APP_ID) // Wayland app_id: pencere kurallarında "mesai" olarak görünür
        .with_inner_size([960.0, 660.0])
        .with_min_inner_size([720.0, 480.0]);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Mesai — Çalışma Takibi",
        options,
        Box::new(|cc| Ok(Box::new(MesaiApp::new(cc)))),
    )
}

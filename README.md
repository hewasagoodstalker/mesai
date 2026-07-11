# Mesai — Çalışma Takip Uygulaması

Ne zaman, ne kadar ve ne üzerinde çalıştığını takip etmek için basit bir masaüstü uygulaması. Rust ile yazıldı; arayüz eframe/egui üzerinden **Wayland'ı doğrudan (native)** kullanır, Wayland yoksa X11'e düşer.

## Özellikler

- **Sayaç:** Başlat, duraklat, devam et, bitir. Süre duvar saatiyle ölçülür; dizüstü uykuya girse bile doğru kalır.
- **Oturum kaydı:** Sayaç bitince konu, not ve etiket girersin.
- **Ekler:** Çalışırken (ya da kayıt formundayken) dosyaları pencereye sürükleyip bırak — kodların oturumla birlikte saklanır. İstersen dosya yolunu elle de yazabilirsin. Dosyalar kopyalanır; orijinali sonradan değişse/silinse bile o günkü hâli sende kalır.
- **İstatistik:** Toplam süre, ortalama oturum, üst üste çalışılan gün serisi, son 14 günün grafiği, konu dağılımı.
- **Dışa aktarma:** Tek tıkla CSV (tablo programları için) veya JSON (tam yedek) — Belgeler klasörüne yazılır.

## Gereksinimler

Rust araç zinciri (1.79 veya üstü yeterli; en kolayı [rustup](https://rustup.rs)):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Çalışma zamanı kütüphaneleri çoğu masaüstü dağıtımda zaten kuruludur:

```sh
# Debian / Ubuntu
sudo apt install libxkbcommon0 libegl1

# Fedora
sudo dnf install libxkbcommon mesa-libEGL

# Arch
sudo pacman -S libxkbcommon mesa
```

## Derleme ve çalıştırma

```sh
cd mesai
cargo run --release
```

Derlenen ikili dosya `target/release/mesai` içindedir; istersen `~/.local/bin/` altına kopyalayıp doğrudan çalıştırabilirsin.

## Veriler nerede?

```
~/.local/share/mesai/
├── data.json          # tüm oturumlar (okunabilir JSON)
└── attachments/<id>/  # her oturumun ek dosyaları
```

Dışa aktarılan dosyalar `~/Belgeler` (yoksa ev dizini) altına `mesai_TARIH.csv` / `.json` adıyla yazılır.

## Wayland notları

- Pencerenin `app_id` değeri `mesai`dir; Sway/Hyprland gibi ortamlarda pencere kurallarında kullanabilirsin.
- `WAYLAND_DISPLAY` tanımlıysa uygulama Wayland'da açılır; X11'e zorlamak istersen `WAYLAND_DISPLAY= cargo run --release`.

## Genişletme fikirleri (istersen)

Kod tek dosyada (`src/main.rs`) ve bölümlere ayrılmış hâlde — kurcalaması kolay. Elini alıştırmak istersen: sayaçsız elle kayıt ekleme, kayıt düzenleme, JSON'dan içe aktarma, haftalık hedef koyma, veriyi SQLite'a taşıma gibi eklentiler güzel egzersiz olur.

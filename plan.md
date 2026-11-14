# Chuyển EqnTide từ Tauri v2 sang desktop UI Rust

## 1. Quyết định đã thống nhất

- Chọn **egui + eframe**: UI và logic desktop bằng Rust, không WebView,
  JavaScript hay MathLive trong tiến trình desktop.
- Đích chuyển đổi là editor công thức trực quan có con trỏ cấu trúc, vùng chọn,
  template, undo/redo; bản LaTeX + preview chỉ là mốc trung gian.
- Giữ engine, CLI, Word/WPS add-in JavaScript/VBA và hợp đồng HTTP tại
  `127.0.0.1:17890`.
- Ưu tiên Linux/Windows; giữ khả năng build macOS, kiểm chứng Office sau.
- Đóng cửa sổ sẽ thoát sau xử lý Lưu/Bỏ thay đổi/Hủy, không thêm tray.
  Bridge CLI/service bên ngoài desktop tiếp tục chạy.
- Giữ Tauri để đối chiếu và quay lui đến khi đạt parity và kiểm thử Office thực tế.

“Pure Rust” áp dụng cho mã UI/logic desktop; vẫn dùng API, driver và thư viện
hệ điều hành. Không yêu cầu viết lại add-in Office bằng Rust.

### Công nghệ

| Lựa chọn | Đánh giá |
| --- | --- |
| egui + eframe — chọn | UI Rust, thuận tiện tùy biến canvas, palette và bảng batch. |
| Iced | State/message rõ ràng, nhưng vẫn cần xây widget editor công thức riêng. |
| Slint | UI khai báo bằng `.slint`, cần cân nhắc giấy phép; ít phù hợp hơn với ưu tiên UI trực tiếp bằng Rust. |

Dùng eframe/egui stable cùng phiên bản, glow, X11/Wayland và AccessKit;
`rfd` cho dialog với portal Linux, `arboard` cho clipboard ảnh,
`interprocess` + `fs2` cho single-instance. Khóa dependency trong Cargo.lock.
Worker thread/channel xử lý chuyển đổi, render và HTTP; không chặn UI thread.

## 2. Kiến trúc và interface

Cargo workspace gồm package engine/CLI gốc làm default member,
`crates/eqntide-editor` không phụ thuộc GUI và `desktop` chứa binary
`eqntide-desktop`. Tách package Tauri khỏi workspace mới trong giai đoạn song song.

Tách service chuyển đổi snippet/document và parser LaTeX dùng chung.
Giữ API chuyển đổi hiện hữu, DTO/endpoint/status/alias HTTP và chữ ký
`WindowCallback`; bổ sung callback mới có thông tin tab nếu cần.
Thêm `tab: Option<String>` cho `CustomUriAction` để đọc `tab=batch` của add-in.

Editor dùng cây có ID ổn định, placeholder, caret và selection.
Command chỉnh sửa là transaction có thể hoàn tác nội dung lẫn selection.
Giữ MathNode và thuộc tính gốc; không round-trip qua LaTeX khi mở/lưu.
Node chưa hỗ trợ được giữ nguyên như khối nguyên vẹn có thể chọn/copy/xóa.
LaTeX là bản nháp: chỉ Apply sau parse thành công, cảnh báo thay cây/thuộc tính;
lỗi không thay nội dung hiện tại.

Renderer tái sử dụng STIX layout/rasterization. API interactive trả ảnh,
kích thước, baseline, vùng node/path và tọa độ caret từ cùng layout.
UI phủ caret/placeholder/selection lên ảnh; cache theo revision/font/zoom/DPI.
Giữ API xuất EMF/PNG và không thêm dependency egui vào engine.

Bridge được kiểm tra `/health`: dùng bridge có sẵn hoặc desktop tự host.
Mỗi session thuộc một bridge duy nhất; thao tác native dùng cùng hợp đồng HTTP.
IPC riêng theo user chuyển argv/URI sang desktop đang chạy, không lấy cổng bridge
làm khóa instance. Không tự lấy session “latest”. Xếp hàng phiên mới thay vì
đè nội dung đang sửa; hỗ trợ khởi động nguội, mở lần hai và focus cửa sổ.

## 3. Các giai đoạn

### Mốc 0 — Baseline và khả thi

Lập bảng parity từ UI cũ: palette/template, phím tắt, font/cỡ chữ, target,
clipboard, session và batch. Corpus gồm phân số, căn, script, giới hạn,
tích phân, delimiter, accent, matrix/cases và mũi tên.
Đo startup, RAM idle, kích thước cài đặt và độ trễ trên cùng máy/profile.
Thử OpenGL, nested formula, caret, DPI, IME tiếng Việt và clipboard Linux/Windows.
Không cam kết cải thiện tài nguyên trước khi có số đo đối chiếu.

### Mốc 1 — Service và desktop shell

Tạo workspace/service, tab Editor/Batch; cửa sổ 940×650, tối thiểu 600×450.
Hoàn thành ownership bridge, single-instance, URI và báo lỗi trước khi thay editor.
Port bốn hướng chuyển đổi DOCX, chọn/drop nhiều tệp, log/kết quả và mở thư mục.
Worker batch tuần tự; Dừng có hiệu lực giữa các tệp. Mặc định xuất bản mới,
chỉ ghi đè khi người dùng chọn rõ ràng.

### Mốc 2 — Editor cấu trúc

Nhập token, đặt caret bằng chuột, chọn vùng, copy/cut/paste, undo/redo.
Trái/phải theo cấu trúc; lên/xuống giữa tử/mẫu, script và ô matrix;
Tab/Shift+Tab qua placeholder. Template bao vùng chọn; port phím tắt MathType
và chỉ kích hoạt khi canvas có focus.
Đạt sửa trực tiếp phân số/căn/script lồng nhau trước khi mở rộng palette.

### Mốc 3 — Parity và Office

Hoàn thành các nhóm template/palette trong baseline, matrix/cases, operator,
accent, delimiter và arrow. Giữ node lạ nguyên vẹn; xác nhận trước Apply LaTeX
có khả năng mất thuộc tính/cấu trúc.
Copy LaTeX/MathML/Flat OPC/PNG; EMF clipboard chỉ dùng khi adapter đã kiểm chứng,
nếu chưa thì xuất `.emf` với nhãn rõ ràng.
Save gửi đúng session ID, MathML, target và cỡ chữ; chờ bridge xác nhận.
Đóng xử lý Lưu/Bỏ thay đổi/Hủy và hủy phiên; chỉ dừng bridge desktop sở hữu.

### Mốc 4 — Đóng gói và cutover

Giữ tên binary, `eqntide://` và `EQNTIDE_DESKTOP_BIN`.
NSIS Windows, DEB/RPM và portable Linux; tài nguyên/add-in/template hoạt động
ngoài repository. Chuyển icon sang assets dùng chung, sửa tên binary cũ trong CI.
Cập nhật README/NOTICE và bỏ yêu cầu WebKit khỏi hướng dẫn native.
Build/smoke macOS trong CI; hoàn thiện URL dispatch và Office sau kiểm chứng.
Chỉ xóa Tauri/frontend khi qua parity và kiểm tra không add-in nào dùng assets đó.

## 4. Kiểm thử và nghiệm thu

- Engine/CLI hiện có; đối chiếu service, native và HTTP theo cấu trúc XML/DOCX.
- Editor: transaction, Unicode, placeholder, vùng chọn, xóa cấu trúc,
  matrix, undo/redo, thuộc tính, node lạ, bản nháp lỗi và giới hạn độ sâu.
- Renderer: hit-test/caret khớp ảnh ở nhiều zoom/DPI; baseline, delimiter,
  ảnh tham chiếu với font nhúng, giới hạn pixel và độ sâu.
- Bridge: open/update/cancel/timeout, nội bộ/ngoài, port bị chiếm,
  session hết hạn, URI nguội/nóng, mở lần hai và không tạo vòng lặp launch.
- Word Windows và WPS Linux/Windows: OMML/MathType, cỡ chữ/baseline và batch.
- Desktop: IME tiếng Việt, clipboard, drag/drop, đường dẫn Unicode,
  dirty close, lỗi worker và không treo trong lúc chuyển đổi.
- Cài/nâng cấp/gỡ, URI và assets trên máy sạch không WebKit.
- CI: `cargo test --workspace --all-targets`, workspace Clippy, cargo-deny;
  manual Linux X11/Wayland và Windows, build/smoke macOS.

Hoàn tất cutover khi không còn phụ thuộc desktop Tauri/WebView/MathLive,
qua bảng parity và không mất dữ liệu trong corpus. Giữ release Tauri cũ để quay lui.
Trạng thái triển khai và các kiểm chứng chưa thực hiện nằm trong
`desktop/VALIDATION.md`; không xem build thành công là đã kiểm chứng Office.

## 5. Nguồn chính thức

- https://github.com/emilk/egui
- https://docs.rs/eframe/latest/eframe/
- https://docs.iced.rs/iced/
- https://slint.dev/faqs
- https://docs.rs/rfd/latest/rfd/
- https://docs.rs/arboard/latest/arboard/

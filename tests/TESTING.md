# EqnTide Testing & AI Visual Inspection Guide

Toàn bộ bài test của repository được tập trung về thư mục `tests/`:

```text
tests/
├── TESTING.md                 # Tài liệu quy trình test & kiểm tra ảnh trực quan cho AI
├── run_all.sh                 # Script chạy toàn bộ test (hỗ trợ --quick, --visual)
├── conversion_matrix.rs       # Ma trận chuyển đổi MathML ↔ OMML ↔ MTEF ↔ OLE ↔ EMF ↔ Flat OPC ↔ LaTeX ↔ PNG/SVG + DOCX + HTTP service
├── cross_conversion.rs        # Regression chuyển đổi chéo OMML ↔ MTEF cho các cấu trúc phức tạp
├── export_verification.rs     # Xuất file chuẩn (.mathml, .omml.xml, .ole, .emf, .png) & bộ ảnh nền trắng cho AI
├── fixtures/
│   ├── bug_toggle_v1.xml      # Fixture XML Word dùng cho test Toggle LaTeX của WPS
│   └── legacy_preview/        # Bộ .ole / .emf legacy (MTEF cũ) kiểm tra upgrade & repair
├── python/
│   ├── test_native_integration.py  # Test đóng gói installer, WPS config merge, Linux service/session
│   ├── test_vba_filetime.py        # Test helper FILETIME cho build_dotm.py
│   └── test_word_ribbon.py         # Test nội dung EqnTide.dotm (VBA stream, class module, Flat OPC cleanup)
└── wps/
    ├── package.json
    ├── test_wps_insertion.js       # Test Insert / Edit / Toggle LaTeX / Taskpane Upgrade (không xuống dòng thừa)
    └── test_wps_ribbon.js          # Test tự động bật tab Ribbon EqnTide khi chọn công thức
```

*(Lưu ý: các unit test nội bộ `#[cfg(test)]` test hàm/struct private vẫn nằm trực tiếp trong `src/`, `crates/`, `desktop/` và được chạy tự động bởi `cargo test --workspace`.)*

---

## Quy trình chạy test chuẩn

### Chạy nhanh toàn bộ bằng script

```bash
./tests/run_all.sh           # Chạy đầy đủ 4 tầng (bao gồm xuất ảnh kiểm tra)
./tests/run_all.sh --quick   # Chạy nhanh Rust + Python + Node (bỏ qua xuất toàn bộ catalog PNG)
./tests/run_all.sh --visual  # Chỉ xuất ảnh nền trắng & in danh sách ảnh để AI/người kiểm tra
```

---

### Tầng 1 — Workspace Unit & Desktop UI Tests

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Bao gồm:
- Core engine (`src/`): parser XML, MathML AST, OMML encoder/decoder, MTEF v3/v5, OLE CFB, EMF/PNG/SVG renderer, LaTeX extensions, HTTP service.
- Editor (`crates/eqntide-editor/`): con trỏ, selection, template wrapping, matrix resize, undo/redo, command completion.
- Desktop UI (`desktop/`): layout 3 hàng palette ở các kích thước 1056×560 / 1100×650 / 1440×900, DPI scaling, live LaTeX debounce, session dirty/save guard.

---

### Tầng 2 — Ma trận chuyển đổi đa định dạng (MathML / OMML / MathType / OLE / EMF / LaTeX / DOCX)

```bash
cargo test --test conversion_matrix --test cross_conversion
```

Kiểm tra:
1. **Format Matrix (`conversion_matrix.rs`)**: Round-trip đầy đủ qua `LaTeX ↔ MathML ↔ OMML ↔ MTEF v5 ↔ OLE (CFB) ↔ EMF (EMR_COMMENT) ↔ Flat OPC (ModernOmml & ModernMathtypeOleEmf) ↔ PNG/SVG` trên 15 nhóm công thức tiêu biểu của toàn bộ 20 palette.
2. **Legacy Fixtures (`tests/fixtures/legacy_preview/`)**: Giải mã `angle`, `fraction`, `system` (`.ole` & `.emf`), kiểm tra repair headerless MTEF và chuyển sang OMML / MathML / LaTeX / PNG / SVG.
3. **DOCX End-to-End**: Tạo `.docx` tổng hợp nhiều công thức, chạy `convert_bytes_with` (EMF, PNG, `EmbedMode::OmmlFallback`), `convert_bytes_to_omml_with` (kèm `math_font`), và `upgrade_legacy_mathtype_bytes_with` (`ModernOmml` và `ModernMathType`).
4. **Bridge Service (`convert_snippet`)**: Mọi cặp `from` (`latex`, `mathml`, `omml`, `flat_opc`, `ole_base64`, `emf_base64`) → `to` (`flat_opc_omml`, `flat_opc_mathtype`, `omml`, `mathml`, `latex`, `emf_base64`, `png_base64`) và mã lỗi `422` cho cấu trúc không hỗ trợ.

---

### Tầng 3 — Office Add-ins (Word `.dotm` & WPS JS) & Native Packaging

**Python (`tests/python/`):**
```bash
python3 -m unittest discover -s tests/python -p "test_*.py" -v
```
*(Nếu `olefile` / `oletools` cài ở bản Python khác, thêm `PYTHONPATH` tương ứng; `tests/run_all.sh` tự dò sẵn đường dẫn này. Cài phụ thuộc test Word DOTM qua `pip install -r word-template/test-requirements.txt`.)*

**WPS Add-in Node.js (`tests/wps/`):**
```bash
node --check wps-addin/main.js
node --check wps-addin/taskpane.js
npm ci --prefix tests/wps
npm test --prefix tests/wps
```

---

### Tầng 4 — Quy trình xuất ảnh cho AI kiểm tra layout trực quan

`render_preview` mặc định sinh PNG **chữ đen nền trong suốt** (tối ưu cho chèn vào Word/WPS, nhưng khó nhìn trên nền tối hoặc khi AI đọc ảnh). Vì vậy `tests/export_verification.rs` có bước hậu xử lý bảng màu PNG (`PLTE` đảo thang xám trên nền trắng đặc, bỏ `tRNS`) và xuất riêng bộ ảnh kiểm tra trực quan vào `target/test_exports/visual/`:

#### Bước 1: Xuất bộ ảnh kiểm tra
```bash
cargo test --test export_verification -- --nocapture
```

Các file đầu ra chính:
- `target/test_exports/visual/INDEX.md`: Mục lục đường dẫn ảnh, kích thước, mô tả và checklist kiểm tra.
- `target/test_exports/visual/14_roundtrip_p1.png` & `14_roundtrip_p2.png`: Bảng so sánh song song `[Orig]` vs `[via OMML]` vs `[via OLE/MTEF]` cho 8 công thức phức tạp để phát hiện sai lệch layout sau chuyển đổi qua lại.
- `target/test_exports/visual/15_layout_edge_cases.png`: Các trường hợp dễ lỗi layout (dấu `\|...\|` quanh phân số, phân số lồng 3 tầng, script gắn vào biểu thức ghép `f'_0^1`, giới hạn tổng/tích xếp chồng, phép chia dài `longdiv`, mũ rộng `widehat`/`overbrace`).
- `target/test_exports/visual/01_*.png` .. `13_*.png`: Ảnh nền trắng của 6 mẫu đơn lẻ và 7 bảng tổng hợp cấu trúc.
- `target/test_exports/visual/catalog/*.png`: Bảng tổng hợp toàn bộ nút bấm của cả 20 nhóm palette.

#### Bước 2: AI xem ảnh và đánh giá layout
Khi cần AI kiểm tra layout (sau khi sửa `src/render/`, `src/omml/`, `src/mtef/`, `src/catalog.rs`):
1. Mở `target/test_exports/visual/14_roundtrip_p1.png` & `14_roundtrip_p2.png` và `target/test_exports/visual/15_layout_edge_cases.png` (cùng các sheet liên quan trong `visual/` hoặc `visual/catalog/`) bằng công cụ xem ảnh (`view_file`).
2. Đối chiếu theo 5 tiêu chí:
   - **Baseline & cân bằng dọc**: Dấu ngoặc trái/phải (`\left\| ... \right\|`, ngoặc nhọn, ngoặc kép) phải cao bằng nhau, cân xứng quanh trục toán học (math axis), không lệch "1 bên cao 1 bên thấp".
   - **Vị trí script & limits**: Chỉ số trên/dưới, `\prescript`, cận tích phân/tổng không đè lên thân ký hiệu hoặc cắt nhau.
   - **Ma trận & hệ phương trình (`cases`)**: Các cột thẳng hàng, ngoặc `{` bao trọn chiều cao tất cả các dòng.
   - **Dấu mũ, gạch ngang & khung (`enclose`)**: Chiều rộng vừa khít nội dung bên dưới/bên trong, nét không bị cụt.
   - **Round-trip parity**: 3 cột `[Orig]`, `[via OMML]`, `[via OLE/MTEF]` trong `14_roundtrip_p1.png` & `14_roundtrip_p2.png` hiển thị cùng cấu trúc toán học tương đương.

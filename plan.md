# Kế hoạch bổ sung đầy đủ bảng công cụ MathType cho EqnTide

> Khôi phục từ phiên làm việc Codex (`Tạo layout app từ phác thảo`) và cập nhật theo trạng thái thực tế của repository.

## 1. Mục tiêu và bố cục

Bổ sung toàn bộ ký hiệu và biến thể mẫu trong 20 ảnh MathType, gồm thao tác chèn, chỉnh sửa trên canvas, gợi ý lệnh LaTeX và xuất MathType/OMML.

- Sidebar dùng danh sách nhóm có cuộn; **Batch DOCX cố định dưới cùng**, ngoài vùng cuộn.
- Giữ vùng công cụ cao đúng **3 dòng**, kích thước nút, khoảng cách và bo góc hiện tại. Nhóm quá nhiều nút được cuộn ngang, không sinh dòng thứ tư.
- Thứ tự 20 nhóm:
  1. Ngoặc (`Fences`)
  2. Phân số & căn (`Fractions`)
  3. Chỉ số (`Scripts`)
  4. Tích phân (`Integrals`)
  5. Tổng (`Sums`)
  6. Tích & phép toán lớn (`BigOperators`)
  7. Ma trận (`Matrices`)
  8. Dấu trên/dưới (`Embellishments`)
  9. Gạch trên/dưới (`Bars`)
  10. Mũi tên có nhãn (`LabeledArrows`)
  11. Khung (`Boxes`)
  12. Quan hệ (`Relations`)
  13. Toán tử (`Operators`)
  14. Mũi tên (`Arrows`)
  15. Logic (`Logic`)
  16. Tập hợp (`Sets`)
  17. Ký hiệu khác (`Symbols`)
  18. Hy Lạp thường (`GreekLower`)
  19. Hy Lạp hoa (`GreekUpper`)
  20. Khoảng trắng & dấu chấm (`Spaces`)
- Giữ các chức năng đang có, gồm những mẫu không xuất hiện trong ảnh. Tooltip dùng tiếng Việt và hiển thị lệnh LaTeX tương ứng.

## 2. Danh mục và thao tác chèn

- Tạo catalog chung ([`src/catalog.rs`](src/catalog.rs)) cho bảng nút, thumbnail và gợi ý canvas. Mỗi mục có ID ổn định, nhóm, tên tiếng Việt, lệnh/alias và mẫu LaTeX/MathML; bỏ việc duy trì danh sách ký hiệu riêng trong completion.
- **Ký hiệu:** bổ sung quan hệ, toán tử, các hướng/kiểu mũi tên, logic, tập hợp, ký hiệu hình học, chữ đặc biệt (`\mathbb`, `\mathcal`, `\mathfrak`), đầy đủ Hy Lạp và các dạng biến thể (`\varepsilon`, `\varphi`, `\vartheta`, `\varpi`, `\varrho`, `\varsigma`, `\varkappa`, `\digamma`, `\varTheta`, `\varUpsilon`, `\lambdabar` nếu thiếu…).
- **Ngoặc:** bổ sung ngoặc một phía, các cặp ngoặc trong ảnh (kể cả nửa mở `[ )`, `( ]`, `] [`, `[ [`, `] ]`, bra `⟨ |`, ket `| ⟩`, ngoặc vuông kép `⟦ ⟧`, `⟦`, `⟧`, bra-ket `⟨ | ⟩`), ngoặc trên/dưới và vị trí nhãn.
- **Phân số, căn, chỉ số:** đủ dạng phân số ngang/chéo/không gạch, kích thước thường/nhỏ, căn có chỉ số, chia dài có/không thương, chỉ số trái/phải và tổ hợp trên/dưới.
- **Tích phân và phép toán lớn:** đủ dạng đơn/kép/ba/vòng trong ảnh; giới hạn dưới, trên, cả hai; vị trí giới hạn bên cạnh (`\nolimits`) hoặc trên/dưới (`\limits`). Áp dụng tương tự cho tổng, tích, đồng tích, hợp và giao lớn.
- **Dấu và đường:** bổ sung dấu chấm (1–4 chấm trên/dưới), mũ, cung, sóng, vector, móc trên/dưới, acute/grave, gạch đơn/kép trên/dưới, gạch xoá và các mũi tên co giãn có nhãn.
- **Ma trận:** bổ sung các mẫu hàng/cột (kể cả 4×4), ma trận có dấu chấm tiếp diễn (`\cdots`, `\vdots`, `\ddots`), `Bmatrix`/`Vmatrix`, và nút **“Ma trận tuỳ chỉnh”** (mặc định 2×2, chọn 1–20 hàng/cột, căn trái/giữa/phải và kiểu ngoặc; huỷ không thay đổi công thức).
- **Khung:** đủ khung kín, bo góc và tổ hợp cạnh hở (`top left`, `top right`, `bottom left`, `bottom right`, `top bottom left`, `top bottom right`, `top left right`, `bottom left right`).

Quy tắc chèn: mẫu có ô trống nhận phần đang chọn vào ô nội dung chính; chỉ số và dấu trang trí gắn vào biểu thức trước con trỏ khi không chọn nội dung (tạo ô trống nếu chưa có); mỗi lần chèn là một bước undo và trả focus về canvas.

## 3. LaTeX, hiển thị và xuất Office

- Dùng cây MathML làm dữ liệu gốc; thumbnail và canvas dùng chung bộ dựng hình.
- Mở rộng parser ([`src/latex_extensions.rs`](src/latex_extensions.rs)) và serializer ([`src/mathml_to_latex.rs`](src/mathml_to_latex.rs)) dùng chung để bảo toàn thuộc tính: kiểu phân số (`\dfrac`, `\tfrac`, `\sfrac`, `\tsfrac`, `\atopfrac`), chỉ số trái (`\prescript`), kiểu chữ (`\mathbb`, `\mathfrak`, `\mathcal`, `\mathbf`, `\mathrm`, `\boldsymbol`), vị trí giới hạn (`\limits`/`\nolimits`), dấu trang trí (`\overaccent`, `\underaccent`, `\doubleoverline`, `\doubleunderline`, …), khung/gạch xoá (`\boxed`, `\cancel`, `\bcancel`, `\xcancel`, `\enclose{notation}{...}`), ma trận có căn cột (`\begin{array}{...}`), `Bmatrix`/`Vmatrix`, và khoảng trắng (`\hspace`, `\,`, `\:`, `\;`, `\quad`, `\qquad`).
- Completion lấy lệnh và alias từ catalog; giữ cách chọn chuột, `↑`/`↓`, `Tab`, `Enter`, `Space`.
- Renderer bổ sung [`src/render/layout/enclosure.rs`](src/render/layout/enclosure.rs) cho khung/gạch xoá, dấu trên/dưới, phân số chéo, khoảng trắng và căn lề ma trận.
- Encoder/decoder MathType MTEF và OMML xử lý đầy đủ các mẫu mới (giới hạn, chỉ số trái, khung, gạch xoá, nhãn mũi tên, phân số chéo/nhỏ, khoảng trắng).
- Cú pháp lỗi báo ở thanh trạng thái, giữ canvas sáng và giữ công thức hợp lệ trước đó. Không đổi giao thức bridge hoặc định dạng phiên hiện tại.

## 4. Trạng thái hiện tại & các hạng mục hoàn thiện

### Đã hoàn tất trong repository
- [x] [`src/catalog.rs`](src/catalog.rs): định nghĩa 20 nhóm, macro `catalog!`, `SYMBOLS`, `ENTRIES`, `PALETTE`, `COMMANDS`, `template()`, `preview_template()`, `attaches_to_previous()`, `matrix()`.
- [x] [`desktop/src/app.rs`](desktop/src/app.rs), [`desktop/src/palette.rs`](desktop/src/palette.rs), [`desktop/src/commands.rs`](desktop/src/commands.rs): sidebar 20 nhóm cuộn độc lập với Batch DOCX ghim dưới đáy, palette 3 dòng cuộn ngang, hộp thoại Ma trận tuỳ chỉnh, completion đọc từ catalog.
- [x] [`src/latex_extensions.rs`](src/latex_extensions.rs) & [`src/mathml_to_latex.rs`](src/mathml_to_latex.rs): mở rộng parser/serializer và đối chiếu ánh xạ ký tự/lệnh LaTeX.
- [x] [`src/render/layout/enclosure.rs`](src/render/layout/enclosure.rs) và cập nhật `layout/*`, `mtef/*`, `omml/*`, `symbols/*`.
- [x] **Sửa lỗi biên dịch, bố cục & bộ chuyển đổi:**
  - [`desktop/src/app.rs`](desktop/src/app.rs): sửa test `batch_stays_visible_while_sidebar_groups_scroll` cho API `egui` và bỏ deprecation warning `Id::new`.
  - [`src/latex_extensions.rs`](src/latex_extensions.rs): dùng chữ cái cho marker `EQNTIDENATIVE...` để `latex2mathml::parse_text` đọc `Token::Letter`; bổ sung parse `\begin{Bmatrix}` / `\begin{Vmatrix}`.
  - [`src/mathml_to_latex.rs`](src/mathml_to_latex.rs): không thêm khoảng trắng thừa sau `\pi` trước `}`; ánh xạ cả `\u{2329}`/`\u{232A}` sang `\langle`/`\rangle`.
  - [`src/render/layout/enclosure.rs`](src/render/layout/enclosure.rs): sửa assertion test `1em` ở `12pt` với `PX_PER_PT = 8.0`.
  - [`src/render/layout/fence.rs`](src/render/layout/fence.rs): hỗ trợ co giãn ngoặc một phía mở (`\left\llbracket ... \right.`, `\left( ... \right.`, `\left| ... \right.`) và bảo toàn `RawHit` trong `stretch_pair`.
  - [`src/mtef/encode.rs`](src/mtef/encode.rs) & [`src/mtef/decode.rs`](src/mtef/decode.rs): nhận diện `<mrow>` ngoặc co giãn từ `latex2mathml` cho `TM_DIRAC`/`TM_INTERVAL`/`TM_OBRACK`, sửa bitmask giải mã `tmINTERVAL` (`tvINTV_LEFT_*`/`tvINTV_RIGHT_*`), và bổ sung dải `0x27C0..=0x27EF`.
- [x] **Đối chiếu đủ 20 ảnh MathType với [`src/catalog.rs`](src/catalog.rs):**
  - Đầy đủ ký hiệu, ngoặc nửa mở `[ )`, `( ]`, `] [`, `[ [`, `] ]`, bra-ket `⟨ |`, `| ⟩`, `⟨ | ⟩`, ngoặc vuông kép `⟦ ⟧`, `⟦`, `⟧`, ma trận `4×4`, mũi tên có nhãn, khung hở, `\lambdabar` và kiểm tra glyph font khả dụng.
- [x] **Kiểm thử toàn diện theo nghiệm thu:**
  - Round-trip `MathML → LaTeX → MathML`, `MathML ↔ MTEF`, `MathML ↔ OMML` và `render_interactive` cho toàn bộ mục catalog, cùng kiểm thử thuộc tính ngữ nghĩa (`\limits`/`\nolimits`, `\dfrac`/`\tfrac`/`\sfrac`/`\tsfrac`/`\atopfrac`, `\prescript`, `mathvariant`, dấu/khung/gạch xoá, ma trận, mũi tên có nhãn, khoảng trắng, ngoặc).
  - Kiểm thử chèn bọc vùng chọn, chèn lồng nhau, điều hướng ô trống, undo/redo, completion, LaTeX lỗi, và bố cục UI đa DPI.
  - Kiểm tra toàn workspace (`cargo fmt --check`, `cargo clippy --workspace --all-targets --locked`, `cargo test --workspace --locked`, `cargo build --workspace --locked`).

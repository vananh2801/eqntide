# Lộ trình Nâng cấp Engine Sắp chữ Toán học (`TYPESETTING_ROADMAP.md`)

Tài liệu này tổng hợp các đề xuất cải tiến bộ dựng ảnh công thức toán học (`src/render/`) theo chuẩn **TeX (Appendix G)**, **OpenType `MATH`** và **Typst**, được chia thành từng hạng mục độc lập để bạn có thể tự triển khai từ từ hoặc giao cho AI Agent thực hiện từng phần.

---

## Kiến trúc hiện tại của `src/render/`

| File | Vai trò hiện tại |
| :--- | :--- |
| `src/render/font.rs` | Nạp các font (`STIXTwoMath-Regular.ttf`, `LiberationSerif`, `OpenSymbol`) qua `ttf-parser`, trích xuất đa giác (`glyph_polys`), hộp mực (`ink`), độ rộng (`advance`), hằng số OpenType `MATH` (`MathConstants`), bảng `MathGlyphInfo` (Italic Correction, Top Accent Attachment, Math Kerning), bảng `MathVariants` cho căn thức, và bảng `GSUB` (`ssty` feature) cho optical size của chỉ số. |
| `src/render/layout/mod.rs` | Chứa trạng thái `Layout`, các hằng số (`AXIS = 0.258`, `RULE = 0.068`, `THIN/MED/THICK`), hệ thống `MathContext` / `MathVariantStyle` ($D, T, S, SS$, `cramped`, `ssty_level`), và hàm duyệt cây `MathNode`. |
| `src/render/layout/row.rs` | Phân loại `Atom` (`Ord`, `Op`, `Bin`, `Rel`, `Open`, `Close`, `Punct`), tính khoảng cách giữa các `Atom` kế nhau, co giãn dấu ngoặc và áp dụng optical size `ssty`. |
| `src/render/layout/fraction.rs` | Dựng phân số (`<mfrac>`) và hệ số nhị thức theo TeX Rule 15 và OpenType `MATH`. |
| `src/render/layout/radical.rs` | Dựng căn bậc hai (`<msqrt>`) và căn bậc $n$ (`<mroot>`) bằng OpenType `MATH` glyph variants (`size1`..`size4`), glyph assembly và định vị bậc căn chuẩn. |
| `src/render/layout/script.rs` | Dựng chỉ số dưới/trên (`<msub>`, `<msup>`, `<msubsup>`, `<mmultiscripts>`) với OpenType math kerning, italic correction và optical size. |
| `src/render/layout/embellish.rs` | Dựng dấu phụ trên/dưới ký tự (`<mover>`, `<munder>`, `<munderover>`). |
| `src/render/layout/delimiter.rs` | Chọn biến thể kích thước (size variants) và lắp ghép mảnh (glyph assembly) cho dấu ngoặc cao từ `STIXTwoMath-Regular.ttf`. |
| `src/render/layout/bigop.rs` | Dựng các toán tử lớn ($\sum, \prod, \bigcup, \bigcap, \lim$). |
| `src/render/layout/integral.rs` | Dựng dấu tích phân ($\int, \iint, \iiint, \oint$). |
| `src/render/layout/matrix.rs` | Dựng ma trận và hệ phương trình (`<mtable>`). |

---

## Danh sách các Hạng mục Cải tiến (Phases)

### Phase 1: Đọc trực tiếp bảng `MATH` từ `STIXTwoMath-Regular.ttf` (✅ Đã hoàn thành)
- **File mục tiêu**: `src/render/font.rs`, `src/render/layout/mod.rs`
- **Vấn đề hiện tại**: Các thông số như `AXIS` (`0.25`), `RULE` (`0.05`), khoảng cách tử/mẫu, độ nâng/hạ chỉ số đang dùng hằng số ước lượng cố định.
- **Giải pháp**:
  1. Sử dụng `face.tables().math` của `ttf-parser` trên `Face::Math` (`STIXTwoMath-Regular.ttf`, `upem = 1000`).
  2. Trích xuất các hằng số từ `math.constants`:
     - `axis_height` (`AxisHeight`)
     - `fraction_rule_thickness` (`FractionRuleThickness`)
     - `fraction_num_display_style_shift_up`, `fraction_num_shift_up`
     - `fraction_den_display_style_shift_down`, `fraction_den_shift_down`
     - `fraction_num_display_style_gap_min`, `fraction_num_gap_min`
     - `fraction_den_display_style_gap_min`, `fraction_den_gap_min`
     - `superscript_shift_up`, `superscript_shift_up_cramped`
     - `subscript_shift_down`, `sub_superscript_gap_min`
     - `radical_vertical_gap`, `radical_display_style_vertical_gap`, `radical_rule_thickness`, `radical_extra_ascender`
  3. Trích xuất thông tin từng glyph từ `math.glyph_info`:
     - `italics_correction(glyph_id) -> Option<f32>`
     - `top_accent_attachment(glyph_id) -> Option<f32>`

---

### Phase 2: Hệ thống 4 cấp `MathStyle` ($D, T, S, SS$) và trạng thái `Cramped` (✅ Đã hoàn thành)
- **File mục tiêu**: `src/render/layout/mod.rs`, `src/render/layout/fraction.rs`, `src/render/layout/radical.rs`, `src/render/layout/script.rs`, `src/render/layout/row.rs`, `src/render/layout/bigop.rs`, `src/render/layout/arrow.rs`, `src/render/layout/integral.rs`, `src/render/layout/matrix.rs`
- **Vấn đề hiện tại**: `Layout::node` chỉ truyền `script: bool` (2 mức kích thước `1.0` và `0.65`), chưa phân biệt `Display`, `Text`, `Script`, `ScriptScript` và trạng thái `cramped` (nén chiều cao mũ khi nằm dưới thanh căn hoặc mẫu số).
- **Giải pháp**:
  1. Thay tham số `script: bool` bằng struct/enum:
     ```rust
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum MathVariantStyle {
         Display,      // D  (100%)
         Text,         // T  (100%)
         Script,       // S  (70% - ScriptPercentScaleDown trong MATH table)
         ScriptScript, // SS (55% - ScriptScriptPercentScaleDown trong MATH table)
     }

     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub struct MathContext {
         pub style: MathVariantStyle,
         pub cramped: bool,
     }
     ```
  2. Triển khai chuyển dịch style theo đúng TeXbook Appendix G:
     - **Phân số**: Tử số chuyển thành style nhỏ hơn 1 cấp (`D -> T`, `T -> S`, giữ nguyên cramped), mẫu số chuyển tương tự nhưng luôn bị `cramped = true`.
     - **Chỉ số trên**: `D, T -> S`; `S, SS -> SS` (giữ nguyên cramped). Dùng `superscript_shift_up_cramped` nếu đang bị cramped.
     - **Chỉ số dưới**: `D, T -> S`; `S, SS -> SS` (luôn `cramped = true`).
     - **Căn thức**: Biểu thức trong căn giữ nguyên cấp style nhưng chuyển thành `cramped = true`.
     - **Bậc căn** ($n$ trong $\sqrt[n]{x}$): Luôn là `ScriptScript` và `cramped = true`.

---

### Phase 3: Chuẩn hóa Baseline và Khoảng cách Phân số theo TeX Rule 15 (✅ Đã hoàn thành)
- **File mục tiêu**: `src/render/layout/fraction.rs`
- **Vấn đề hiện tại**: Tử số và mẫu số đang căn chỉnh thủ công dựa trên `asc` và `desc` thực tế của từng biểu thức con, dẫn đến các phân số đơn giản (như $\frac{1}{2}$ và $\frac{a}{b}$) có đường gạch ngang bị lệch trục hoặc khoảng cách không đồng đều khi đặt cạnh nhau.
- **Giải pháp**:
  1. Đặt thanh phân số chính xác tại trục toán học `axis = c.axis_height * em`.
  2. Độ dịch baseline của tử số và mẫu số được cố định theo thông số `MATH`:
     - Display style: dịch lên `fraction_num_display_style_shift_up * em`, dịch xuống `fraction_den_display_style_shift_down * em`.
     - Text/Script style: dịch lên `fraction_num_shift_up * em`, dịch xuống `fraction_den_shift_down * em`.
  3. Kiểm tra điều kiện khe hở tối thiểu (`gap_min`): Chỉ khi biểu thức con quá lớn chạm vào thanh gạch thì mới đẩy dãn ra thêm.
  4. Kết quả: Các phân số thông thường đặt cạnh nhau luôn có tử số và mẫu số thẳng hàng tuyệt đối.

---

### Phase 4: Italic Correction, Math Kerning & Top Accent Attachment (✅ Đã hoàn thành)
- **File mục tiêu**: `src/render/layout/script.rs`, `src/render/layout/embellish.rs`, `src/render/layout/row.rs`
- **Vấn đề hiện tại**:
  - Chỉ số trên/dưới đang dùng offset ngang cố định (`0.02 * em`, riêng chữ `'f'` cộng thêm `0.10 * em`).
  - Dấu phụ ($\hat{x}$, $\dot{A}$, $\tilde{f}$) căn giữa theo chiều rộng hộp cơ sở (`base.w / 2.0`), nên trên chữ nghiêng dấu mũ trông hơi lệch sang trái so với đỉnh chữ.
- **Giải pháp**:
  1. **Trong `script.rs`**:
     - Lấy `ic = f.italic_correction(last_glyph)` từ bảng `MATH`.
     - Lấy `sup_kern` / `sub_kern` từ bảng `MathKernInfo` (kiểm tra `TopRightMathKern` / `BottomLeftMathKern` cho chỉ số trên và `BottomRightMathKern` / `TopLeftMathKern` cho chỉ số dưới theo hàm số độ cao `script_y`).
     - Đặt chỉ số trên tại `sup_x = base.w + ic + sup_kern + 0.02 * em` và chỉ số dưới tại `sub_x = base.w + 0.02 * em + sub_kern` (kéo nhẹ chỉ số dưới vào hốc nghiêng dưới chân chữ như $P_1, V_0, \int_a^b$).
  2. **Trong `embellish.rs`**:
     - Nếu cơ sở là 1 ký tự đơn, lấy `attach_x = f.top_accent_attachment(base_glyph)` từ bảng `MATH` (hoặc xấp xỉ `base.w / 2.0 + 0.35 * ic`) để đặt tâm dấu mũ đúng đỉnh nghiêng của chữ ($\hat{x}, \vec{v}, \dot{y}$).

---

### Phase 5: Dựng dấu Căn thức (`\u{221A}`) bằng Glyph Variants & Assembly từ `STIXTwoMath` (✅ Đã hoàn thành)
- **File mục tiêu**: `src/render/layout/radical.rs`, `src/render/font.rs`
- **Vấn đề hiện tại**: Dấu căn $\sqrt{}$ trong `radical.rs` đang được vẽ thủ công bằng 3 đoạn thẳng nối nhau.
- **Giải pháp**:
  1. Tái sử dụng bảng `MathVariants` của OpenType `MATH` trên ký tự `\u{221A}`:
     - Với biểu thức dưới căn có chiều cao vừa và nhỏ: chọn glyph biến thể có sẵn (`uni221A`, `uni221A.s1`, `uni221A.s2`, `uni221A.s3`) từ font `STIXTwoMath-Regular.ttf`.
     - Với biểu thức dưới căn rất cao (như căn của phân số hoặc ma trận nhiều tầng): dùng bảng `GlyphAssembly` của `U+221A` để ghép phần móc căn (`uni221A.s4`) + các đoạn thẳng đứng kéo dài (`uni221A.x`) + góc nối (`uni221A.t`) với thanh ngang (`vinculum`) phía trên.
  2. Định vị bậc căn (`deg` trong $\sqrt[n]{x}$) theo đúng thông số `RadicalDegreeBottomRaisePercent` ($55\%$) và `RadicalKernBeforeDegree` ($0.065 \times em$) / `RadicalKernAfterDegree` ($-0.335 \times em$) của bảng `MATH`.

---

### Phase 6: Optical Size (`ssty` GSUB feature) cho Chỉ số trên/dưới (✅ Đã hoàn thành)
- **File mục tiêu**: `src/render/font.rs`, `src/render/layout/row.rs`, `src/render/layout/script.rs`
- **Vấn đề trước đây**: Khi thu nhỏ glyph xuống $65\%–70\%$ cho chỉ số trên/dưới, nét chữ bị mảnh đi tương ứng.
- **Giải pháp đã thực hiện**:
  - Đọc bảng `GSUB` của `STIXTwoMath-Regular.ttf` để tra cứu feature `ssty` (`Script Style`):
    - Cấp `Script` ($S$): dùng alternate glyph số 1 (`ssty = 1`, biến thể `.ssty`).
    - Cấp `ScriptScript` ($SS$): dùng alternate glyph số 2 (`ssty = 2`, biến thể `.ssty2`).
  - Toàn bộ 529 cặp alternate glyphs được nạp sẵn vào bảng tra cứu; áp dụng xuyên suốt từ bounding box, advance width, rasterizer canvas PNG đến các vector path trong file EMF.
  - Các glyph này có nét dày dặn và tỷ lệ bề ngang thoáng hơn, đảm bảo độ đậm (optical weight) đồng nhất với chữ cơ sở khi thu nhỏ.

---

## ⚠️ Nguyên tắc Bản quyền & Đọc Mã nguồn Tham khảo (Dành cho AI Agent & Lập trình viên)

Khi tham khảo mã nguồn từ các dự án mã nguồn mở (ví dụ: Typst, KaTeX, MathJax, Chromium MathML, TeX, v.v.) để triển khai các Phase trong lộ trình này, **bắt buộc tuân thủ nghiêm ngặt các nguyên tắc sau**:

1. **Hiểu thuật toán — Không sao chép nguyên văn (Clean-room Implementation)**:
   - AI Agent chỉ được đọc mã nguồn mở nhằm mục đích **hiểu giải pháp kỹ thuật, công thức toán học và luồng thuật toán** (ví dụ: cách tra cứu bảng OpenType `MATH`, cách chọn size variants cho dấu ngoặc, các bước căn chỉnh baseline của TeX...).
   - **Tuyệt đối KHÔNG sao chép (copy-paste) nguyên khối mã nguồn**, cấu trúc file hay tên hàm/biến mang tính bản quyền từ các dự án khác vào repo này. Mã nguồn phải được viết lại độc lập bằng Rust, tương thích tự nhiên với kiến trúc module `LBox` / `MathNode` hiện có của dự án.
   - Khi không chắc một đoạn code có quá giống nguồn hay không, viết lại từ đầu.

2. **Quy định khi Port hoặc Tham chiếu Cụ thể**:
   - Trường hợp cần port trực tiếp một thuật toán đặc thù, bảng ánh xạ ký tự hoặc hằng số từ một dự án có giấy phép phần mềm tương thích (MIT, BSD, Apache 2.0):
     - **BẮT BUỘC** phải giữ nguyên hoặc chú thích rõ thông báo bản quyền (copyright notice) cùng nguồn gốc của tác giả gốc ngay tại đầu file hoặc tại hàm/struct liên quan.
     - **BẮT BUỘC** phải bổ sung ghi nhận trích nguồn và điều khoản giấy phép tương ứng vào file [NOTICE](NOTICE) và mục References của [README.md](README.md) để tránh rủi ro pháp lý.
   - **Tuyệt đối KHÔNG** sao chép hoặc port mã nguồn từ các dự án mang giấy phép Copyleft mạnh (GPL, AGPL) vì có nguy cơ làm lây lan giấy phép và xung đột với giấy phép [MIT](LICENSE) của repo.

3. **Mọi đóng góp (kể cả do AI Agent sinh ra) phải tương thích để repo giữ nguyên giấy phép MIT.**

---

## Tài liệu Tham khảo Chuẩn

1. **Donald E. Knuth — *The TeXbook*, Appendix G: "Generating Boxes from Formulas"** (Rules 1–22)
   - Quy chiếu gốc cho mọi thuật toán khoảng cách `Atom` (`mu` spacing), phân số, căn thức, toán tử lớn và chỉ số.
2. **Microsoft OpenType Specification — The `MATH` Table**
   - URL: https://learn.microsoft.com/en-us/typography/opentype/spec/math
   - Đặc tả chi tiết về `MathConstants`, `MathGlyphInfo`, `MathVariants` và `GlyphAssembly`.
3. **Typst Math Layout Engine (`typst-layout/src/math/`)**
   - URL: https://github.com/typst/typst/tree/main/crates/typst-layout/src/math
   - Mã nguồn Rust tham khảo trực tiếp cách kết hợp `ttf-parser` + bảng OpenType `MATH` để sắp chữ công thức toán học.
4. **W3C MathML Core Specification — Layout Algorithms**
   - URL: https://w3c.github.io/mathml-core/
   - Mô tả từng bước (step-by-step) kèm sơ đồ hình học cho từng cấu trúc `<mfrac>`, `<msqrt>`, `<msubsup>`, `<mover>`, `<munder>`.
5. **Ulrik Vieth — *\"OpenType Math Illuminated\"*** (*TUGboat*, Vol. 30, 2009)
   - URL: https://tug.org/TUGboat/tb30-1/tb94vieth.pdf
   - Bảng ánh xạ giữa 22 tham số `\fontdimen` của TeX và các trường trong bảng OpenType `MATH`.

---

## Tổng kết Lộ trình

Tất cả 6 Phase nâng cấp bộ dựng sắp chữ toán học TeX / OpenType MATH đã được hoàn thành 100%:
- **Phase 1**: Hằng số OpenType `MATH` từ `STIXTwoMath-Regular.ttf`.
- **Phase 2**: Hệ thống 4 cấp `MathStyle` ($D, T, S, SS$) và trạng thái `Cramped`.
- **Phase 3**: Chuẩn hóa baseline & khoảng cách phân số theo TeX Rule 15.
- **Phase 4**: Italic correction, OpenType math kerning & top accent attachment.
- **Phase 5**: Glyph variants và vertical glyph assembly cho dấu căn thức $\sqrt{}$.
- **Phase 6**: Optical size qua tính năng `ssty` trong bảng OpenType `GSUB`.

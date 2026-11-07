# Lộ trình Nâng cấp Engine Sắp chữ Toán học (`TYPESETTING_ROADMAP.md`)

Tài liệu này tổng hợp các đề xuất cải tiến bộ dựng ảnh công thức toán học (`src/render/`) theo chuẩn **TeX (Appendix G)**, **OpenType `MATH`** và **Typst**, được chia thành từng hạng mục độc lập để bạn có thể tự triển khai từ từ hoặc giao cho AI Agent thực hiện từng phần.

---

## Kiến trúc hiện tại của `src/render/`

| File | Vai trò hiện tại |
| :--- | :--- |
| `src/render/font.rs` | Nạp các font (`STIXTwoMath-Regular.ttf`, `LiberationSerif`, `OpenSymbol`) qua `ttf-parser`, trích xuất đa giác (`glyph_polys`), hộp mực (`ink`) và độ rộng (`advance`). |
| `src/render/layout/mod.rs` | Chứa trạng thái `Layout`, các hằng số (`SCRIPT = 0.65`, `AXIS = 0.25`, `RULE = 0.05`, `THIN/MED/THICK`) và hàm duyệt cây `MathNode`. |
| `src/render/layout/row.rs` | Phân loại `Atom` (`Ord`, `Op`, `Bin`, `Rel`, `Open`, `Close`, `Punct`), tính khoảng cách giữa các `Atom` kế nhau và co giãn dấu ngoặc. |
| `src/render/layout/fraction.rs` | Dựng phân số (`<mfrac>`). |
| `src/render/layout/radical.rs` | Dựng căn bậc hai (`<msqrt>`) và căn bậc $n$ (`<mroot>`). |
| `src/render/layout/script.rs` | Dựng chỉ số dưới/trên (`<msub>`, `<msup>`, `<msubsup>`, `<mmultiscripts>`). |
| `src/render/layout/embellish.rs` | Dựng dấu phụ trên/dưới ký tự (`<mover>`, `<munder>`, `<munderover>`). |
| `src/render/layout/delimiter.rs` | Chọn biến thể kích thước (size variants) và lắp ghép mảnh (glyph assembly) cho dấu ngoặc cao từ `STIXTwoMath-Regular.ttf`. |
| `src/render/layout/bigop.rs` | Dựng các toán tử lớn ($\sum, \prod, \bigcup, \bigcap, \lim$). |
| `src/render/layout/integral.rs` | Dựng dấu tích phân ($\int, \iint, \iiint, \oint$). |
| `src/render/layout/matrix.rs` | Dựng ma trận và hệ phương trình (`<mtable>`). |

---

## Danh sách các Hạng mục Cải tiến (Phases)

### Phase 1: Đọc trực tiếp bảng `MATH` từ `STIXTwoMath-Regular.ttf`
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

### Phase 2: Hệ thống 4 cấp `MathStyle` ($D, T, S, SS$) và trạng thái `Cramped`
- **File mục tiêu**: `src/render/layout/mod.rs`, `src/render/layout/fraction.rs`, `src/render/layout/radical.rs`, `src/render/layout/script.rs`
- **Vấn đề hiện tại**: `Layout::node` chỉ truyền `script: bool` (2 mức kích thước `1.0` và `0.65`), chưa phân biệt `Display`, `Text`, `Script`, `ScriptScript` và trạng thái `cramped` (nén chiều cao mũ khi nằm dưới thanh căn hoặc mẫu số).
- **Giải pháp**:
  1. Thay tham số `script: bool` bằng struct/enum:
     ```rust
     #[derive(Clone, Copy, PartialEq, Eq)]
     pub enum MathVariantStyle {
         Display,      // D  (100%)
         Text,         // T  (100%)
         Script,       // S  (70% - ScriptPercentScaleDown trong MATH table)
         ScriptScript, // SS (55% - ScriptScriptPercentScaleDown trong MATH table)
     }

     #[derive(Clone, Copy)]
     pub struct MathContext {
         pub style: MathVariantStyle,
         pub cramped: bool,
     }
     ```
  2. Quy tắc chuyển trạng thái chuẩn TeX (Appendix G):
     - Biểu thức dưới căn (`<msqrt>`, `<mroot>`): giữ nguyên `style`, bật `cramped = true`.
     - Tử số phân số: chuyển `D -> T`, `T -> S`, `S -> SS`, giữ nguyên `cramped`.
     - Mẫu số phân số: chuyển `D -> T`, `T -> S`, `S -> SS`, bật `cramped = true`.
     - Chỉ số trên (`sup`): chuyển `D | T -> S`, `S | SS -> SS`, giữ nguyên `cramped`.
     - Chỉ số dưới (`sub`): chuyển `D | T -> S`, `S | SS -> SS`, bật `cramped = true`.

---

### Phase 3: Chuẩn hóa Baseline Tử số & Mẫu số Phân số (TeX Rule 15b–15d)
- **File mục tiêu**: `src/render/layout/fraction.rs`
- **Vấn đề hiện tại**: Tử số và mẫu số đang đặt cách thanh phân số một khoảng cố định `gap = 0.12 * em` tính từ đáy hộp mực (`num.desc`) và đỉnh hộp mực (`den.asc`). Khi hai phân số đặt cạnh nhau như $\frac{a}{b} + \frac{g}{y}$, chữ $g$ có chân thò xuống (`desc > 0`) nên bị đẩy cao hơn chữ $a$, làm hai tử số không thẳng hàng trên cùng một baseline.
- **Giải pháp (TeX Rule 15 / OpenType MATH)**:
  1. Đặt baseline mặc định của tử số tại `u = num_shift_up` (tính từ baseline phương trình) và mẫu số tại `v = den_shift_down`.
  2. Kiểm tra khoảng hở thực tế từ đáy tử số tới mặt trên thanh phân số:
     - `actual_num_gap = (u - num.desc) - (axis + thick / 2.0)`
     - Nếu `actual_num_gap < min_num_gap`, tăng `u += min_num_gap - actual_num_gap`.
  3. Kiểm tra khoảng hở thực tế từ mặt dưới thanh phân số tới đỉnh mẫu số:
     - `actual_den_gap = (v - den.asc) - (thick / 2.0 - axis)`
     - Nếu `actual_den_gap < min_den_gap`, tăng `v += min_den_gap - actual_den_gap`.
  4. Kết quả: Các phân số thông thường đặt cạnh nhau luôn có tử số và mẫu số thẳng hàng tuyệt đối.

---

### Phase 4: Italic Correction, Math Kerning & Top Accent Attachment
- **File mục tiêu**: `src/render/layout/script.rs`, `src/render/layout/embellish.rs`, `src/render/layout/row.rs`
- **Vấn đề hiện tại**:
  - Chỉ số trên/dưới đang dùng offset ngang cố định (`0.02 * em`, riêng chữ `'f'` cộng thêm `0.10 * em`).
  - Dấu phụ (`\hat{x}`, `\dot{A}`, `\tilde{f}`) căn giữa theo chiều rộng hộp cơ sở (`base.w / 2.0`), nên trên chữ nghiêng dấu mũ trông hơi lệch sang trái so với đỉnh chữ.
- **Giải pháp**:
  1. **Trong `script.rs`**:
     - Lấy `ic = f.italic_correction(last_glyph)` từ bảng `MATH`.
     - Đặt chỉ số trên tại `sup_x = base.w + ic` và chỉ số dưới tại `sub_x = base.w - sub_kern` (kéo nhẹ chỉ số dưới vào hốc nghiêng dưới chân chữ như $P_i, V_0, \int_a^b$).
  2. **Trong `embellish.rs`**:
     - Nếu cơ sở là 1 ký tự đơn, lấy `attach_x = f.top_accent_attachment(base_glyph)` từ bảng `MATH` (hoặc xấp xỉ `base.w / 2.0 + 0.35 * ic`) để đặt tâm dấu mũ đúng đỉnh nghiêng của chữ.

---

### Phase 5: Dựng dấu Căn thức (`\u{221A}`) bằng Glyph Variants & Assembly từ `STIXTwoMath`
- **File mục tiêu**: `src/render/layout/radical.rs`, `src/render/layout/delimiter.rs`
- **Vấn đề hiện tại**: Dấu căn $\sqrt{}$ trong `radical.rs` đang được vẽ thủ công bằng 3 đoạn thẳng nối nhau.
- **Giải pháp**:
  1. Tái sử dụng cơ chế `MathVariants` trong `delimiter.rs` cho ký tự `'\u{221A}'` (`Face::Math`):
     - Với biểu thức dưới căn có chiều cao vừa và nhỏ: chọn glyph biến thể có sẵn (`size1`..`size4`) trong `STIXTwoMath-Regular.ttf`.
     - Với biểu thức dưới căn rất cao (như căn của phân số hoặc ma trận): dùng bảng `GlyphAssembly` của `U+221A` trong `STIXTwoMath` để ghép phần móc căn + đoạn thẳng đứng kéo dài + góc nối với thanh ngang (`vinculum`) phía trên.
  2. Định vị bậc căn (`deg` trong $\sqrt[n]{x}$) theo đúng thông số `RadicalDegreeBottomRaisePercent` (`60%`) và `RadicalKernBeforeDegree` / `RadicalKernAfterDegree` của bảng `MATH`.

---

### Phase 6: Optical Size (`ssty` GSUB feature) cho Chỉ số trên/dưới
- **File mục tiêu**: `src/render/font.rs`
- **Vấn đề hiện tại**: Khi thu nhỏ glyph xuống $65\%–70\%$ cho chỉ số trên/dưới, nét chữ cũng bị mảnh đi tương ứng.
- **Giải pháp**:
  - Đọc bảng `GSUB` của `STIXTwoMath-Regular.ttf` để tra cứu feature `ssty` (`Script Style`):
    - Cấp `Script` ($S$): dùng alternate glyph số 1 (`ssty = 1`).
    - Cấp `ScriptScript` ($SS$): dùng alternate glyph số 2 (`ssty = 2`).
  - Các glyph này được thiết kế với nét dày hơn và khoảng trắng thoáng hơn để khi thu nhỏ vẫn có độ đậm (optical weight) đồng nhất với chữ chính.

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
5. **Ulrik Vieth — *"OpenType Math Illuminated"*** (*TUGboat*, Vol. 30, 2009)
   - URL: https://tug.org/TUGboat/tb30-1/tb94vieth.pdf
   - Bảng ánh xạ giữa 22 tham số `\fontdimen` của TeX và các trường trong bảng OpenType `MATH`.

---

## Gợi ý Prompt khi giao việc cho AI Agent

Khi muốn nhờ AI Agent thực hiện từng phần, bạn chỉ cần nhắn:
> *"Hãy đọc file `TYPESETTING_ROADMAP.md` và triển khai **Phase 1 & Phase 3** (đọc `MathConstants` từ `STIXTwoMath-Regular.ttf` và chuẩn hóa baseline phân số theo Rule 15 của TeX), sau đó chạy `cargo test` và render thử `src.docx` để đối chiếu."*

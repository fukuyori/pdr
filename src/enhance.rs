//! スキャン画像PDF向けの表示時補正。
//!
//! pdfium で画像化したページを描画前に補正する。
//! グレースケール(Luma8)へ落とすため、補正適用時は描画も軽くなる。

use image::{DynamicImage, GrayImage, imageops};

/// 表示時の画像補正設定。強度は 0（無効）〜100（最大）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Enhance {
    /// ヒストグラム伸長結果を原画像へ混ぜる割合。
    pub contrast: u8,
    /// アンシャープマスクの強度。
    pub sharpen: u8,
    /// ページごとの暗部と背景を目標レベルへ寄せる。
    pub auto_level: bool,
    /// 大津法による白黒二値化。指定時は他の補正を適用しない。
    pub binarize: bool,
}

impl Enhance {
    pub const NONE: Self = Self {
        contrast: 0,
        sharpen: 0,
        auto_level: false,
        binarize: false,
    };

    pub const fn new(contrast: u8, sharpen: u8, binarize: bool) -> Self {
        Self {
            contrast: if contrast > 100 { 100 } else { contrast },
            sharpen: if sharpen > 100 { 100 } else { sharpen },
            auto_level: false,
            binarize,
        }
    }
}

/// 補正を適用して返す。
pub fn apply_enhance(image: DynamicImage, enhance: Enhance) -> DynamicImage {
    if enhance.binarize {
        return DynamicImage::ImageLuma8(binarize(image.to_luma8()));
    }

    if enhance.contrast == 0 && enhance.sharpen == 0 {
        return image;
    }

    let mut gray = image.to_luma8();
    if enhance.contrast > 0 {
        gray = if enhance.auto_level {
            auto_level_adjust(gray, enhance.contrast)
        } else {
            contrast_stretch_amount(gray, enhance.contrast)
        };
    }
    if enhance.sharpen > 0 {
        gray = unsharp_mask(gray, enhance.sharpen);
    }
    DynamicImage::ImageLuma8(gray)
}

/// 輝度ヒストグラムの両端2%を外れ値として切り、[lo,hi]→[0,255] に線形伸長する。
/// スキャン特有の「灰ばんだ背景」を白に飛ばし、薄い文字を締める。
pub fn contrast_stretch(gray: GrayImage) -> GrayImage {
    contrast_stretch_amount(gray, 100)
}

/// コントラスト伸長した画像を、指定した強度で元画像へ合成する。
pub fn contrast_stretch_amount(mut gray: GrayImage, amount: u8) -> GrayImage {
    let amount = amount.min(100) as u16;
    if amount == 0 {
        return gray;
    }

    let (hist, total) = histogram(&gray);
    let cut = (total as f32 * 0.02) as u32;

    let mut acc = 0u32;
    let mut lo = 0usize;
    for v in 0..256 {
        acc += hist[v];
        if acc > cut {
            lo = v;
            break;
        }
    }
    acc = 0;
    let mut hi = 255usize;
    for v in (0..256).rev() {
        acc += hist[v];
        if acc > cut {
            hi = v;
            break;
        }
    }
    if hi <= lo {
        return gray;
    }

    let range = (hi - lo) as f32;
    let mut lut = [0u8; 256];
    for (v, slot) in lut.iter_mut().enumerate() {
        *slot = (((v as f32 - lo as f32) / range) * 255.0).clamp(0.0, 255.0) as u8;
    }
    for p in gray.pixels_mut() {
        let original = p[0] as u16;
        let stretched = lut[p[0] as usize] as u16;
        p[0] = ((original * (100 - amount) + stretched * amount + 50) / 100) as u8;
    }
    gray
}

/// 半径1pxのぼかしとの差分を加算し、文字の輪郭を強調する。
/// 強度100で差分を2倍して加算する。
pub fn unsharp_mask(mut gray: GrayImage, amount: u8) -> GrayImage {
    let amount = amount.min(100) as f32;
    if amount == 0.0 {
        return gray;
    }

    let blurred = imageops::blur(&gray, 1.0);
    let gain = amount * 0.02;
    for (pixel, blurred_pixel) in gray.pixels_mut().zip(blurred.pixels()) {
        let original = pixel[0] as f32;
        let detail = original - blurred_pixel[0] as f32;
        pixel[0] = (original + detail * gain).round().clamp(0.0, 255.0) as u8;
    }
    gray
}

/// ページごとの輝度分布から文字と背景のレベルを推定し、目標濃度へ寄せる。
/// 背景が一様でないページは写真を含む可能性があるため、補正を自動的に弱める。
pub fn auto_level_adjust(mut gray: GrayImage, amount: u8) -> GrayImage {
    const TARGET_DARK: f32 = 20.0;
    const TARGET_BACKGROUND: f32 = 245.0;

    let requested_amount = amount.min(100) as f32;
    if requested_amount == 0.0 {
        return gray;
    }

    let (hist, total) = histogram(&gray);
    let dark = percentile(&hist, total, 10); // 下位1%
    let background = percentile(&hist, total, 950); // 下位95%
    if background.saturating_sub(dark) < 24 || dark > 200 {
        return gray;
    }

    // 文字中心のページは、背景色付近に多数の画素が集まる。
    let background_floor = background.saturating_sub(16) as usize;
    let background_pixels: u32 = hist[background_floor..].iter().sum();
    let background_ratio = background_pixels as f32 / total.max(1) as f32;
    let confidence = if background_ratio >= 0.35 {
        1.0
    } else if background_ratio >= 0.15 {
        0.5
    } else {
        0.25
    };
    let blend = requested_amount * confidence / 100.0;
    let source_range = (background - dark) as f32;

    for pixel in gray.pixels_mut() {
        let original = pixel[0] as f32;
        let normalized = ((original - dark as f32) / source_range).clamp(0.0, 1.0);
        let target = TARGET_DARK + normalized * (TARGET_BACKGROUND - TARGET_DARK);
        pixel[0] = (original + (target - original) * blend)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    gray
}

/// ヒストグラムの指定パーミル位置（0〜1000）の輝度を返す。
fn percentile(hist: &[u32; 256], total: u32, permille: u32) -> u8 {
    let target = ((total as u64 * permille.min(1000) as u64) / 1000).max(1) as u32;
    let mut accumulated = 0u32;
    for (value, count) in hist.iter().enumerate() {
        accumulated += count;
        if accumulated >= target {
            return value as u8;
        }
    }
    255
}

/// 大津法で閾値を求め、白黒に二値化する。
pub fn binarize(mut gray: GrayImage) -> GrayImage {
    let (hist, total) = histogram(&gray);
    let thr = otsu_threshold(&hist, total);
    for p in gray.pixels_mut() {
        p[0] = if p[0] > thr { 255 } else { 0 };
    }
    gray
}

fn histogram(gray: &GrayImage) -> ([u32; 256], u32) {
    let mut hist = [0u32; 256];
    for p in gray.pixels() {
        hist[p[0] as usize] += 1;
    }
    (hist, (gray.width() * gray.height()).max(1))
}

/// クラス間分散を最大化する閾値（大津の手法）を返す。
fn otsu_threshold(hist: &[u32; 256], total: u32) -> u8 {
    let sum: f64 = (0..256).map(|i| i as f64 * hist[i] as f64).sum();
    let mut sum_b = 0.0f64;
    let mut w_b = 0u32;
    let mut max_between = 0.0f64;
    let mut thr = 0u8;
    for t in 0..256 {
        w_b += hist[t];
        if w_b == 0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0 {
            break;
        }
        sum_b += t as f64 * hist[t] as f64;
        let m_b = sum_b / w_b as f64;
        let m_f = (sum - sum_b) / w_f as f64;
        let between = w_b as f64 * w_f as f64 * (m_b - m_f).powi(2);
        if between > max_between {
            max_between = between;
            thr = t as u8;
        }
    }
    thr
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    /// 二峰性（暗64と明192）の画像は、その中間あたりで二値化される。
    #[test]
    fn otsu_splits_bimodal() {
        let mut img = GrayImage::new(4, 2);
        for (i, p) in img.pixels_mut().enumerate() {
            *p = Luma([if i % 2 == 0 { 64 } else { 192 }]);
        }
        let out = binarize(img);
        let vals: Vec<u8> = out.pixels().map(|p| p[0]).collect();
        assert!(vals.iter().all(|&v| v == 0 || v == 255));
        assert!(vals.contains(&0) && vals.contains(&255));
    }

    /// コントラスト伸長後は最小0・最大255に張り付く。
    #[test]
    fn contrast_uses_full_range() {
        let mut img = GrayImage::new(10, 10);
        for (i, p) in img.pixels_mut().enumerate() {
            // 100〜150 の狭い範囲に分布
            *p = Luma([100 + (i % 50) as u8]);
        }
        let out = contrast_stretch(img);
        let min = out.pixels().map(|p| p[0]).min().unwrap();
        let max = out.pixels().map(|p| p[0]).max().unwrap();
        assert_eq!(min, 0);
        assert_eq!(max, 255);
    }

    #[test]
    fn zero_strength_preserves_image() {
        let img = GrayImage::from_fn(5, 5, |x, y| Luma([32 + (x + y) as u8]));
        assert_eq!(contrast_stretch_amount(img.clone(), 0), img);
        assert_eq!(unsharp_mask(img.clone(), 0), img);
    }

    #[test]
    fn unsharp_mask_emphasizes_local_detail() {
        let mut img = GrayImage::from_pixel(9, 9, Luma([128]));
        img.put_pixel(4, 4, Luma([160]));

        let out = unsharp_mask(img, 50);

        assert!(out.get_pixel(4, 4)[0] > 160);
    }

    #[test]
    fn auto_level_normalizes_pages_with_different_density() {
        let light_page = GrayImage::from_fn(10, 10, |x, _| Luma([if x < 2 { 120 } else { 230 }]));
        let dark_page = GrayImage::from_fn(10, 10, |x, _| Luma([if x < 2 { 70 } else { 190 }]));

        let light_out = auto_level_adjust(light_page, 100);
        let dark_out = auto_level_adjust(dark_page, 100);

        assert_eq!(light_out.get_pixel(0, 0)[0], 20);
        assert_eq!(dark_out.get_pixel(0, 0)[0], 20);
        assert_eq!(light_out.get_pixel(9, 0)[0], 245);
        assert_eq!(dark_out.get_pixel(9, 0)[0], 245);
    }

    #[test]
    fn auto_level_preserves_low_contrast_blank_page() {
        let img = GrayImage::from_pixel(10, 10, Luma([240]));
        assert_eq!(auto_level_adjust(img.clone(), 100), img);
    }
}

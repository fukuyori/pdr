//! スキャン画像PDF向けの表示時補正。
//!
//! pdfium で画像化したページを描画前に 1 パス補正する。
//! グレースケール(Luma8)へ落とすため、補正適用時は描画も軽くなる。

use image::{DynamicImage, GrayImage};

/// 補正モード
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Enhance {
    /// 補正なし（原画像のまま）
    None,
    /// コントラスト伸長（背景の灰ばみを飛ばして文字を締める）
    Contrast,
    /// 大津法による白黒二値化
    Binarize,
}

/// 補正を適用して返す。
pub fn apply_enhance(image: DynamicImage, mode: Enhance) -> DynamicImage {
    match mode {
        Enhance::None => image,
        Enhance::Contrast => DynamicImage::ImageLuma8(contrast_stretch(image.to_luma8())),
        Enhance::Binarize => DynamicImage::ImageLuma8(binarize(image.to_luma8())),
    }
}

/// 輝度ヒストグラムの両端2%を外れ値として切り、[lo,hi]→[0,255] に線形伸長する。
/// スキャン特有の「灰ばんだ背景」を白に飛ばし、薄い文字を締める。
pub fn contrast_stretch(mut gray: GrayImage) -> GrayImage {
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
        p[0] = lut[p[0] as usize];
    }
    gray
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
}

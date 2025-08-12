use egui::Color32;
use fastrand::Rng;
use image::{
    DynamicImage,
    GenericImageView,
    imageops::FilterType
};

use super::color::{
    lerp_srgb,
    hsv_to_rgb,
    rgb_to_hsv
};

#[inline]
fn sqr_dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

pub fn extract_palette(img: &DynamicImage, k: usize) -> [Color32; 3] {
    let k = k.max(2).min(6);
    let (w, h) = img.dimensions();
    let scale = 64.0 / (w.max(h) as f32);
    let dw = ((w as f32) * scale).max(16.0) as u32;
    let dh = ((h as f32) * scale).max(16.0) as u32;
    let small = img.resize(dw, dh, FilterType::Triangle).to_rgba8();

    let mut samples: Vec<[f32; 3]> = Vec::with_capacity((dw * dh) as usize);
    for p in small.pixels() {
        let [r, g, b, a] = p.0;
        if a >= 16 {
            samples.push([r as f32, g as f32, b as f32]);
        }
    }
    if samples.is_empty() {
        return [
            Color32::from_rgb(28, 28, 30),
            Color32::from_rgb(20, 20, 22),
            Color32::from_rgb(10, 10, 12),
        ];
    }
    if samples.len() > 4096 {
        samples.truncate(4096);
    }

    let mut rng = Rng::with_seed(0xCAFEBABE_u64);
    let mut centers: Vec<[f32; 3]> = Vec::with_capacity(k);
    centers.push(samples[rng.usize(0..samples.len())]);
    while centers.len() < k {
        let dists: Vec<f32> = samples.iter().map(|c| {
            centers.iter().map(|m| sqr_dist(*c, *m)).fold(f32::INFINITY, f32::min)
        }).collect();
        let sum: f32 = dists.iter().sum::<f32>().max(1.0);
        let mut pick = rng.f32() * sum;
        let mut idx = 0;
        for (i, d) in dists.iter().enumerate() {
            if pick <= *d {
                idx = i;
                break;
            }
            pick -= *d;
        }
        centers.push(samples[idx]);
    }

    for _ in 0..10 {
        let mut acc = vec!([0.0f32; 3]); acc.resize(k, [0.0; 3]);
        let mut cnt = vec![0u32; k];
        for s in &samples {
            let mut best = 0usize;
            let mut bd = f32::INFINITY;
            for (i, c) in centers.iter().enumerate() {
                let d = sqr_dist(*s, *c);
                if d < bd {
                    bd = d;
                    best = i;
                }
            }
            acc[best][0] += s[0];
            acc[best][1] += s[1];
            acc[best][2] += s[2];
            cnt[best] += 1;
        }
        for i in 0..k {
            if cnt[i] > 0 {
                centers[i][0] = acc[i][0] / cnt[i] as f32;
                centers[i][1] = acc[i][1] / cnt[i] as f32;
                centers[i][2] = acc[i][2] / cnt[i] as f32;
            }
        }
    }

    let mut counts = vec![0u32; k];
    for s in &samples {
        let mut best = 0usize;
        let mut bd = f32::INFINITY;
        for (i, c) in centers.iter().enumerate() {
            let d = sqr_dist(*s, *c);
            if d < bd {
                bd = d;
                best = i;
            }
        }
        counts[best] += 1;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(counts[i]));

    let c0 = centers[idx[0]];
    let c1 = centers[*idx.get(1).unwrap_or(&idx[0])];
    let c2 = centers[*idx.get(2).unwrap_or(&idx[0])];

    let to = |v: [f32; 3]| Color32::from_rgb(
        v[0].clamp(0.0, 255.0) as u8,
        v[1].clamp(0.0, 255.0) as u8,
        v[2].clamp(0.0, 255.0) as u8,
    );

    [to(c0), to(c1), to(c2)]
}

pub fn make_gradient_stops(pal: [Color32; 3]) -> [Color32; 3] {
    let mid = pal[1];
    let mid_d = lerp_srgb(mid, Color32::from_rgb(0, 0, 0), 0.28);
    let top = lerp_srgb(mid_d, Color32::from_rgb(255, 255, 255), 0.05);
    let bottom = lerp_srgb(mid_d, Color32::from_rgb(0, 0, 0), 0.34);
    [top, mid_d, bottom]
}

pub fn accent_from_palette(pal: [Color32; 3]) -> Color32 {
    let mut best = pal[0];
    let mut best_score = 0.0;
    for &c in &pal {
        let (_h, s, v) = rgb_to_hsv(c);
        let score = s * (0.7 + 0.3 * v);
        if score > best_score {
            best_score = score;
            best = c;
        }
    }
    let (h, mut s, mut v) = rgb_to_hsv(best);
    s = (s * 1.10).clamp(0.0, 1.0);
    v = (v * 1.02).clamp(0.0, 1.0);
    hsv_to_rgb(h, s, v.max(0.78))
}

pub fn title_from_accent(accent: Color32) -> Color32 {
    let (h, mut s, mut v) = rgb_to_hsv(accent);
    s = (s * 0.85).clamp(0.0, 1.0);
    v = v.max(0.94);
    hsv_to_rgb(h, s, v)
}

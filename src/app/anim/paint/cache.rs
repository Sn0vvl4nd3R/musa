use std::cell::RefCell;

pub(super) struct BgCache {
    pub(super) nx: usize,
    pub(super) ny: usize,
    pub(super) cols: usize,
    pub(super) rows: usize,

    pub(super) u: Vec<f32>,
    pub(super) v: Vec<f32>,

    pub(super) sin_09u: Vec<f32>,
    pub(super) cos_09u: Vec<f32>,
    pub(super) sin_06u: Vec<f32>,
    pub(super) cos_06u: Vec<f32>,
    pub(super) sin_07v: Vec<f32>,
    pub(super) cos_07v: Vec<f32>,
    pub(super) sin_08v: Vec<f32>,
    pub(super) cos_08v: Vec<f32>,

    pub(super) w_pad: usize,
    pub(super) iso: Vec<f32>,
    pub(super) tmp: Vec<f32>,
    pub(super) row_bias: Vec<f32>,

    pub(super) indices: Vec<u32>,
}

impl BgCache {
    #[inline(always)]
    pub(super) fn new() -> Self {
        Self {
            nx: 0,
            ny: 0,
            cols: 0,
            rows: 0,

            u: Vec::new(),
            v: Vec::new(),

            sin_09u: Vec::new(),
            cos_09u: Vec::new(),
            sin_06u: Vec::new(),
            cos_06u: Vec::new(),
            sin_07v: Vec::new(),
            cos_07v: Vec::new(),
            sin_08v: Vec::new(),
            cos_08v: Vec::new(),

            w_pad: 0,
            iso: Vec::new(),
            tmp: Vec::new(),
            row_bias: Vec::new(),

            indices: Vec::new(),
        }
    }

    #[inline]
    pub(super) fn ensure(&mut self, nx: usize, ny: usize) {
        if self.nx == nx && self.ny == ny {
            return;
        }

        debug_assert!(nx > 0 && ny > 0);

        self.nx = nx;
        self.ny = ny;

        let cols = nx + 1;
        let rows = ny + 1;
        self.cols = cols;
        self.rows = rows;

        self.u.resize(cols, 0.0);
        self.v.resize(rows, 0.0);

        let inv_nx = 1.0 / nx as f32;
        let inv_ny = 1.0 / ny as f32;

        for (i, u) in self.u.iter_mut().enumerate() {
            *u = (i as f32) * inv_nx;
        }
        for (j, v) in self.v.iter_mut().enumerate() {
            *v = (j as f32) * inv_ny;
        }

        self.sin_09u.resize(cols, 0.0);
        self.cos_09u.resize(cols, 0.0);
        self.sin_06u.resize(cols, 0.0);
        self.cos_06u.resize(cols, 0.0);

        for i in 0..cols {
            let u = self.u[i];
            let (s09, c09) = (0.9 * u).sin_cos();
            let (s06, c06) = (0.6 * u).sin_cos();
            self.sin_09u[i] = s09;
            self.cos_09u[i] = c09;
            self.sin_06u[i] = s06;
            self.cos_06u[i] = c06;
        }

        self.sin_07v.resize(rows, 0.0);
        self.cos_07v.resize(rows, 0.0);
        self.sin_08v.resize(rows, 0.0);
        self.cos_08v.resize(rows, 0.0);

        for j in 0..rows {
            let v = self.v[j];
            let (s07, c07) = (0.7 * v).sin_cos();
            let (s08, c08) = (0.8 * v).sin_cos();
            self.sin_07v[j] = s07;
            self.cos_07v[j] = c07;
            self.sin_08v[j] = s08;
            self.cos_08v[j] = c08;
        }

        self.w_pad = cols + 2;
        let h_pad = rows + 2;
        let cap = h_pad * self.w_pad;

        self.iso.resize(cap, 0.0);
        self.tmp.resize(cap, 0.0);
        self.row_bias.resize(rows, 0.0);

        let tri_count = nx * ny * 2;
        let index_count = tri_count * 3;

        self.indices.clear();
        self.indices.resize(index_count, 0);

        let mut idx = 0usize;
        for j in 0..ny {
            let row0 = j * cols;
            let row1 = row0 + cols;
            for i in 0..nx {
                let i0 = (row0 + i) as u32;
                let i1 = i0 + 1;
                let i2 = (row1 + i) as u32;
                let i3 = i2 + 1;

                self.indices[idx] = i0;
                self.indices[idx + 1] = i2;
                self.indices[idx + 2] = i1;
                self.indices[idx + 3] = i1;
                self.indices[idx + 4] = i2;
                self.indices[idx + 5] = i3;

                idx += 6;
            }
        }
    }

    #[inline(always)]
    pub(super) fn fill_padding_for(buf: &mut [f32], w_pad: usize, rows: usize, cols: usize) {
        let h_pad = rows + 2;

        debug_assert_eq!(buf.len(), h_pad * w_pad);
        debug_assert!(cols + 2 <= w_pad);

        unsafe {
            let ptr = buf.as_mut_ptr();

            let top_src = 1 * w_pad + 1;
            let top_dst = 0 * w_pad + 1;
            std::ptr::copy_nonoverlapping(ptr.add(top_src), ptr.add(top_dst), cols);

            let bot_src = rows * w_pad + 1;
            let bot_dst = (h_pad - 1) * w_pad + 1;
            std::ptr::copy_nonoverlapping(ptr.add(bot_src), ptr.add(bot_dst), cols);
        }

        for j in 1..=rows {
            let row = j * w_pad;
            let left_src = row + 1;
            let right_src = row + cols;
            buf[row + 0] = buf[left_src];
            buf[row + (w_pad - 1)] = buf[right_src];
        }

        // Углы.
        buf[0] = buf[1 * w_pad + 1];
        buf[w_pad - 1] = buf[1 * w_pad + cols];
        buf[(h_pad - 1) * w_pad + 0] = buf[rows * w_pad + 1];
        buf[(h_pad - 1) * w_pad + (w_pad - 1)] = buf[rows * w_pad + cols];
    }

    #[inline]
    pub(super) fn blur_iso(&mut self, passes: usize) {
        if passes == 0 {
            return;
        }

        let w = self.w_pad;
        let rows = self.rows;
        let cols = self.cols;

        let iso = &mut self.iso;
        let tmp = &mut self.tmp;

        let h_pad = rows + 2;
        debug_assert_eq!(iso.len(), h_pad * w);
        debug_assert_eq!(tmp.len(), h_pad * w);

        for _ in 0..passes {
            Self::fill_padding_for(iso, w, rows, cols);
            for j in 1..=rows {
                let row = j * w;
                for i in 1..=cols {
                    let idx = row + i;
                    let center = iso[idx];
                    let sum_lr = iso[idx - 1] + iso[idx + 1];
                    tmp[idx] = sum_lr.mul_add(0.25, center * 0.5);
                }
            }

            Self::fill_padding_for(tmp, w, rows, cols);
            for j in 1..=rows {
                let row = j * w;
                let row_above = (j - 1) * w;
                let row_below = (j + 1) * w;

                for i in 1..=cols {
                    let idx = row + i;
                    let center = tmp[idx];
                    let sum_ud = tmp[row_above + i] + tmp[row_below + i];
                    iso[idx] = sum_ud.mul_add(0.25, center * 0.5);
                }
            }
        }
    }
}

thread_local! {
    pub(super) static BG_CACHE: RefCell<BgCache> = RefCell::new(BgCache::new());
}

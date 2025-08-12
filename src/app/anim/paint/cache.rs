use std::cell::RefCell;

pub(super) struct BgCache {
    pub(super) nx: usize,
    pub(super) ny: usize,
    pub(super) cols: usize,
    pub(super) rows: usize,

    pub(super) u: Vec<f32>,
    pub(super) v: Vec<f32>,

    pub(super) sin_09u: Vec<f32>, pub(super) cos_09u: Vec<f32>,
    pub(super) sin_06u: Vec<f32>, pub(super) cos_06u: Vec<f32>,
    pub(super) sin_07v: Vec<f32>, pub(super) cos_07v: Vec<f32>,
    pub(super) sin_08v: Vec<f32>, pub(super) cos_08v: Vec<f32>,

    pub(super) w_pad: usize,
    pub(super) iso: Vec<f32>,
    pub(super) tmp: Vec<f32>,
    pub(super) row_bias: Vec<f32>,

    pub(super) indices: Vec<u32>,
}

impl BgCache {
    pub(super) fn new() -> Self {
        Self {
            nx: 0, ny: 0, cols: 0, rows: 0,
            u: Vec::new(), v: Vec::new(),
            sin_09u: Vec::new(), cos_09u: Vec::new(),
            sin_06u: Vec::new(), cos_06u: Vec::new(),
            sin_07v: Vec::new(), cos_07v: Vec::new(),
            sin_08v: Vec::new(), cos_08v: Vec::new(),
            w_pad: 0,
            iso: Vec::new(), tmp: Vec::new(), row_bias: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub(super) fn ensure(&mut self, nx: usize, ny: usize) {
        if self.nx == nx && self.ny == ny {
            return;
        }

        self.nx = nx;
        self.ny = ny;
        self.cols = nx + 1;
        self.rows = ny + 1;

        self.u.resize(self.cols, 0.0);
        self.v.resize(self.rows, 0.0);
        for i in 0..=nx {
            self.u[i] = i as f32 / nx as f32;
        }
        for j in 0..=ny {
            self.v[j] = j as f32 / ny as f32;
        }

        self.sin_09u.resize(self.cols, 0.0);
        self.cos_09u.resize(self.cols, 0.0);
        self.sin_06u.resize(self.cols, 0.0);
        self.cos_06u.resize(self.cols, 0.0);
        for i in 0..=nx {
            let u = self.u[i];
            self.sin_09u[i] = (0.9 * u).sin();
            self.cos_09u[i] = (0.9 * u).cos();
            self.sin_06u[i] = (0.6 * u).sin();
            self.cos_06u[i] = (0.6 * u).cos();
        }
        self.sin_07v.resize(self.rows, 0.0);
        self.cos_07v.resize(self.rows, 0.0);
        self.sin_08v.resize(self.rows, 0.0);
        self.cos_08v.resize(self.rows, 0.0);
        for j in 0..=ny {
            let v = self.v[j];
            self.sin_07v[j] = (0.7 * v).sin();
            self.cos_07v[j] = (0.7 * v).cos();
            self.sin_08v[j] = (0.8 * v).sin();
            self.cos_08v[j] = (0.8 * v).cos();
        }

        self.w_pad = self.cols + 2;
        let cap = (self.rows + 2) * self.w_pad;
        self.iso.resize(cap, 0.0);
        self.tmp.resize(cap, 0.0);
        self.row_bias.resize(self.rows, 0.0);

        self.indices.clear();
        self.indices.reserve(nx * ny * 6);
        for j in 0..ny {
            for i in 0..nx {
                let i0 = (j * self.cols + i) as u32;
                let i1 = i0 + 1;
                let i2 = i0 + self.cols as u32;
                let i3 = i2 + 1;
                self.indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
            }
        }
    }

    pub(super) fn fill_padding_for(buf: &mut [f32], w_pad: usize, rows: usize, cols: usize) {
        let h_pad = rows + 2;

        for i in 1..=cols {
            let top_src = 1 * w_pad + i;
            buf[0 * w_pad + i] = buf[top_src];
            let bot_src = rows * w_pad + i;
            buf[(h_pad - 1) * w_pad + i] = buf[bot_src];
        }
        for j in 1..=rows {
            let left_src  = j * w_pad + 1;
            let right_src = j * w_pad + cols;
            buf[j * w_pad + 0] = buf[left_src];
            buf[j * w_pad + (w_pad - 1)] = buf[right_src];
        }

        buf[0] = buf[1 * w_pad + 1];
        buf[w_pad - 1] = buf[1 * w_pad + cols];
        buf[(h_pad - 1) * w_pad + 0] = buf[rows * w_pad + 1];
        buf[(h_pad - 1) * w_pad + (w_pad-1)] = buf[rows * w_pad + cols];
    }

    pub(super) fn blur_iso(&mut self, passes: usize) {
        let w = self.w_pad;
        let rows = self.rows;
        let cols = self.cols;

        for _ in 0..passes {
            Self::fill_padding_for(&mut self.iso, w, rows, cols);
            for j in 1..=rows {
                let row = j * w;
                for i in 1..=cols {
                    let idx = row + i;
                    self.tmp[idx] = (self.iso[idx - 1] + 2.0 * self.iso[idx] + self.iso[idx + 1]) * 0.25;
                }
            }

            Self::fill_padding_for(&mut self.tmp, w, rows, cols);
            for j in 1..=rows {
                let row = j * w;
                for i in 1..=cols {
                    let idx = row + i;
                    self.iso[idx] = (self.tmp[idx - w] + 2.0 * self.tmp[idx] + self.tmp[idx + w]) * 0.25;
                }
            }
        }
    }
}

thread_local! {
    pub(super) static BG_CACHE: RefCell<BgCache> = RefCell::new(BgCache::new());
}

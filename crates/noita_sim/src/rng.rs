#[derive(Clone, Debug)]
pub struct NollaPrng {
    pub world_seed: u32,
    seed: i32,
}

impl NollaPrng {
    pub fn new(world_seed: u32) -> Self {
        Self {
            world_seed,
            seed: world_seed as i32,
        }
    }

    fn set_random_seed_helper(r: f64) -> u32 {
        let e = r.to_bits() & 0x7fff_ffff_ffff_ffff;
        let c: i64 = if r < 0.0 { -1 } else { 1 };
        let f = (e & 0x000f_ffff_ffff_ffff) | 0x0010_0000_0000_0000;
        let g = 0x433_u64.wrapping_sub(e >> 0x34);
        let h = if g >= 64 { 0 } else { f >> g };
        let exp_hi = ((e >> 0x20) as u32) >> 0x14;
        let j = (!(if 0x433 < exp_hi { 1_u32 } else { 0_u32 })).wrapping_add(1);
        let a = ((j as u64) << 0x20) | j as u64;
        let b = (((!a & h) | ((f << 0x0d) & a)) as i64).wrapping_mul(c);
        (b as u64 & 0xffff_ffff) as u32
    }

    fn set_random_seed_helper_int(r: i64) -> u32 {
        let dr = r as f64;
        let e = dr.to_bits() & 0x7fff_ffff_ffff_ffff;
        let c: i64 = if r < 0 { -1 } else { 1 };
        let f = (e & 0x000f_ffff_ffff_ffff) | 0x0010_0000_0000_0000;
        let g = 0x433_u64.wrapping_sub(e >> 0x34);
        let h = if g >= 64 { 0 } else { f >> g };
        let exp_hi = ((e >> 0x20) as u32) >> 0x14;
        let j = (!(if 0x433 < exp_hi { 1_u32 } else { 0_u32 })).wrapping_add(1);
        let a = ((j as u64) << 0x20) | j as u64;
        let b = (((!a & h) | ((f << 0x0d) & a)) as i64).wrapping_mul(c);
        (b as u64 & 0xffff_ffff) as u32
    }

    fn helper2(a: u32, b: u32, ws: u32) -> u32 {
        let mut u2 = a.wrapping_sub(b).wrapping_sub(ws) ^ (ws >> 0x0d);
        let mut u1 = b.wrapping_sub(u2).wrapping_sub(ws) ^ (u2 << 8);
        let mut u3 = ws.wrapping_sub(u2).wrapping_sub(u1) ^ (u1 >> 0x0d);
        u2 = u2.wrapping_sub(u1).wrapping_sub(u3) ^ (u3 >> 0x0c);
        u1 = u1.wrapping_sub(u2).wrapping_sub(u3) ^ (u2 << 0x10);
        u3 = u3.wrapping_sub(u2).wrapping_sub(u1) ^ (u1 >> 5);
        u2 = u2.wrapping_sub(u1).wrapping_sub(u3) ^ (u3 >> 3);
        u1 = u1.wrapping_sub(u2).wrapping_sub(u3) ^ (u2 << 10);
        u3.wrapping_sub(u2).wrapping_sub(u1) ^ (u1 >> 0x0f)
    }

    pub fn set_random_seed(&mut self, x: f64, y: f64) {
        let ws = self.world_seed;
        let a = ws ^ 0x9326_2e6f;
        let b = (a & 0xfff) as f64;
        let c = ((a >> 0x0c) & 0xfff) as f64;
        let x_ = x + b;
        let mut y_ = y + c;
        let mut r = x_ * 134_217_727.0;
        let e = Self::set_random_seed_helper(r);
        let ax = f64::from_bits(x_.to_bits() & 0x7fff_ffff_ffff_ffff);
        let ay = f64::from_bits(y_.to_bits() & 0x7fff_ffff_ffff_ffff);
        if 102_400.0 <= ay || ax <= 1.0 {
            r = y_ * 134_217_727.0;
        } else {
            let mut y2 = y_ * 3483.328;
            y2 += e as f64;
            y_ *= y2;
            r = y_;
        }
        let f = Self::set_random_seed_helper(r);
        let g = Self::helper2(e, f, ws);
        let mut s = g as f64;
        s /= 4_294_967_295.0;
        s *= 2_147_483_639.0;
        s += 1.0;
        self.seed = s as i32;
        self.next_f32();
        for _ in 0..(ws & 3) {
            self.next_f32();
        }
    }

    pub fn set_random_seed_int(&mut self, x: i32, y: i32) {
        let ws = self.world_seed;
        let a = ws ^ 0x9326_2e6f;
        let b = (a & 0xfff) as i32;
        let c = ((a >> 0x0c) & 0xfff) as i32;
        let x_ = x.wrapping_add(b);
        let y_ = y.wrapping_add(c);
        let mut r = (x_ as i64).wrapping_mul(134_217_727_i64);
        let e = Self::set_random_seed_helper_int(r);
        if y_.abs() >= 102_400 || x_.abs() <= 1 {
            r = (y_ as i64).wrapping_mul(134_217_727_i64);
        } else {
            let mut y2 = y_ as f64 * 3483.328;
            y2 += e as f64;
            r = (y_ as f64 * y2) as i64;
        }
        let f = Self::set_random_seed_helper_int(r);
        let g = Self::helper2(e, f, ws);
        let mut s = g as f64;
        s /= 4_294_967_295.0;
        s *= 2_147_483_639.0;
        s += 1.0;
        self.seed = s as i32;
        self.next_f32();
        for _ in 0..(ws & 3) {
            self.next_f32();
        }
    }

    fn advance(&mut self) -> i32 {
        let q = self.seed / 0x1f31d;
        let mut v = self
            .seed
            .wrapping_mul(0x41a7)
            .wrapping_add(q.wrapping_mul(-0x7fff_ffff));
        if v < 0 {
            v = v.wrapping_add(0x7fff_ffff);
        }
        self.seed = v;
        v
    }

    pub fn next_f32(&mut self) -> f32 {
        self.advance() as f32 / 0x7fff_ffff_u32 as f32
    }
    pub fn next_f64(&mut self) -> f64 {
        self.advance() as f64 / 0x7fff_ffff_u32 as f64
    }
    pub fn random_f32(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next_f32()
    }

    pub fn random_i32_inclusive(&mut self, a: i32, b: i32) -> i32 {
        let seed = self.advance();
        a + (((b - a + 1) as f64 * seed as f64 * 4.656_612_875e-10) as i32)
    }

    fn distribution(&mut self, mean: f32, sharpness: f32, baseline: f32) -> f32 {
        for _ in 0..100 {
            let r1 = self.next_f32();
            let r2 = self.next_f32();
            let div = (r1 - mean).abs();
            if r2 < (1.0 - div) * baseline {
                return r1;
            }
            if div < 0.5 {
                #[allow(clippy::approx_constant)]
                let v11 = (((0.5 - mean) + r1) * 3.1415).sin();
                let v12 = v11.powf(sharpness);
                if v12 > r2 {
                    return r1;
                }
            }
        }
        self.next_f32()
    }

    pub fn random_distribution_i32(
        &mut self,
        min: i32,
        max: i32,
        mean: i32,
        sharpness: f32,
    ) -> i32 {
        if sharpness == 0.0 {
            return self.random_i32_inclusive(min, max);
        }
        let adj_mean = (mean - min) as f32 / (max - min) as f32;
        let v = self.distribution(adj_mean, sharpness, 0.005);
        min + ((max - min) as f32 * v).round() as i32
    }

    pub fn random_distribution_f32(
        &mut self,
        min: f32,
        max: f32,
        mean: f32,
        sharpness: f32,
    ) -> f32 {
        if sharpness == 0.0 {
            return self.next_f32() * (max - min) + min;
        }
        let adj_mean = (mean - min) / (max - min);
        min + (max - min) * self.distribution(adj_mean, sharpness, 0.005)
    }
}

const CHUNK: usize = 128;
const HEIGHT: usize = 16;
type HV = bool;

use cgmath::{Point3, Vector3};
use cgmath::InnerSpace;

pub struct HeightChunk {
    data: [[[HV; CHUNK]; HEIGHT]; CHUNK]
}
impl HeightChunk {
    pub fn value(value: HV) -> Self {
        Self {
            data: [[[value; CHUNK]; HEIGHT]; CHUNK]
        }
    }

    pub fn for_each<T: Fn(usize, usize, usize) -> HV>(f: T) -> Self {
        let mut data = [[[false; CHUNK]; HEIGHT]; CHUNK];
        for (x,row) in data.iter_mut().enumerate() {
            for (y,col) in row.iter_mut().enumerate() {
                for (z,cell) in col.iter_mut().enumerate() {
                    *cell = f(x,y,z);
                }
            }
        };
        return HeightChunk { data }
    }

    pub fn get(&self, c: (usize, usize, usize)) -> Option<&HV> {
        return self.data.get(c.0).and_then(|row| row.get(c.1)).and_then(|row| row.get(c.2));
    }
    pub fn get_mut(&mut self, c: (usize, usize, usize)) -> Option<&mut HV> {
        return self.data.get_mut(c.0).and_then(|row| row.get_mut(c.1)).and_then(|row| row.get_mut(c.2))
    }

    pub fn to_index(p: cgmath::Point3<f32>) -> (usize, usize, usize) {
        (p.x.round() as usize,p.y.round() as usize,p.z.round() as usize)
    }

    pub fn getp(&self, p: cgmath::Point3<f32>) -> Option<&HV> {
        return self.get(Self::to_index(p));
    }
    pub fn is_empty(&self, p: cgmath::Point3<f32>) -> bool {
        ! *self.getp(p).unwrap_or(&false)
    }
    pub fn is_empty_raw(&self, c: (usize,usize,usize)) -> bool {
        ! *self.get(c).unwrap_or(&false)
    }

    pub fn ray(&self, start: Point3<f32>, ray: Vector3<f32>) -> Option<((usize,usize,usize),HV)> {
        let mut dist: f32 = 0.;
        loop {
            let point = start + ray.normalize() * dist;
            if *self.getp(point).unwrap_or(&false) {
                return Some((Self::to_index(point), *self.getp(point).unwrap()))
            }
            
            if dist < ray.magnitude() {
                dist += 1.;
            } else {
                return None
            }
        }
    }

    pub fn positions(&self) -> Vec<[f32; 3]> {
        let mut pos = Vec::<[f32;3]>::new();
        for (x, row) in self.data.iter().enumerate() {
            for (y, col) in row.iter().enumerate() {
                for (z, cell) in col.iter().enumerate() {
                    if *cell && self.is_empty_raw((x,y + 1,z)) { pos.push([x as f32,y as f32,z as f32]) }
                }
            }
        }
        return pos
    }
}
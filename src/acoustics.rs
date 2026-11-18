use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::RwLock;

pub const MATERIAL_AIR: u8 = 0;
pub const MATERIAL_DOOR_WOOD: u8 = 1;
pub const MATERIAL_DOOR_IRON: u8 = 2;
pub const MATERIAL_DOOR_HEAVY: u8 = 3;
pub const MATERIAL_WALL_WOOD: u8 = 4;
pub const MATERIAL_WALL_SOLID: u8 = 255;

pub struct AcousticMap {
    pub width: usize,
    pub height: usize,
    pub max_z: usize,
    pub tiles: Vec<u8>,
}

impl AcousticMap {
    pub fn new(width: usize, height: usize, max_z: usize) -> Self {
        let size = width.saturating_mul(height).saturating_mul(max_z);
        Self {
            width,
            height,
            max_z,
            tiles: vec![MATERIAL_AIR; size],
        }
    }

    #[inline]
    fn get_index(&self, x: usize, y: usize, z: usize) -> Option<usize> {
        if x == 0 || x > self.width || y == 0 || y > self.height || z == 0 || z > self.max_z {
            return None;
        }
        let z_offset = (z - 1) * self.width * self.height;
        let y_offset = (y - 1) * self.width;
        let x_offset = x - 1;
        Some(z_offset + y_offset + x_offset)
    }

    pub fn set_tile(&mut self, x: usize, y: usize, z: usize, material: u8) {
        if let Some(idx) = self.get_index(x, y, z) {
            self.tiles[idx] = material;
        }
    }

    pub fn get_tile(&self, x: usize, y: usize, z: usize) -> u8 {
        if let Some(idx) = self.get_index(x, y, z) {
            self.tiles[idx]
        } else {
            MATERIAL_WALL_SOLID
        }
    }
}

static MAP: RwLock<Option<AcousticMap>> = RwLock::new(None);

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    energy: i16,
    x: usize,
    y: usize,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        self.energy.cmp(&other.energy)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

const LOCAL_DIM: usize = 31;
const LOCAL_CENTER: isize = 15;

/// Runs sound wave BFS from (src_x, src_y) on level z with given voice_range.
/// Evaluates volume percent (0..=100) for each target in `targets`.
pub fn evaluate_acoustics(
    map: &AcousticMap,
    z: usize,
    src_x: usize,
    src_y: usize,
    voice_range: usize,
    targets: &[(usize, usize)],
) -> Vec<u8> {
    if targets.is_empty() {
        return Vec::new();
    }

    if voice_range == 0 {
        return vec![0; targets.len()];
    }

    let initial_energy: i16 = (voice_range as i16) * 10;
    let mut energy_grid = [-1i16; LOCAL_DIM * LOCAL_DIM];

    let mut heap = BinaryHeap::new();

    let center_idx = (LOCAL_CENTER as usize) * LOCAL_DIM + (LOCAL_CENTER as usize);
    energy_grid[center_idx] = initial_energy;
    heap.push(State {
        energy: initial_energy,
        x: src_x,
        y: src_y,
    });

    let dirs: [(isize, isize); 8] = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];

    while let Some(State { energy, x, y }) = heap.pop() {
        let dx_from_center = (x as isize) - (src_x as isize);
        let dy_from_center = (y as isize) - (src_y as isize);
        let lx = dx_from_center + LOCAL_CENTER;
        let ly = dy_from_center + LOCAL_CENTER;

        if lx < 0 || lx >= LOCAL_DIM as isize || ly < 0 || ly >= LOCAL_DIM as isize {
            continue;
        }

        let current_idx = (ly as usize) * LOCAL_DIM + (lx as usize);
        if energy < energy_grid[current_idx] {
            continue;
        }

        for (dx, dy) in dirs {
            let nx_i = (x as isize) + dx;
            let ny_i = (y as isize) + dy;

            if nx_i < 1
                || nx_i > (map.width as isize)
                || ny_i < 1
                || ny_i > (map.height as isize)
            {
                continue;
            }

            let n_dx_from_center = nx_i - (src_x as isize);
            let n_dy_from_center = ny_i - (src_y as isize);
            let n_lx = n_dx_from_center + LOCAL_CENTER;
            let n_ly = n_dy_from_center + LOCAL_CENTER;

            if n_lx < 0 || n_lx >= LOCAL_DIM as isize || n_ly < 0 || n_ly >= LOCAL_DIM as isize {
                continue;
            }

            let nx = nx_i as usize;
            let ny = ny_i as usize;

            // Prevent diagonal sound cutting through solid wall corners
            if dx != 0 && dy != 0 {
                let ortho1 = map.get_tile((x as isize + dx) as usize, y, z);
                let ortho2 = map.get_tile(x, (y as isize + dy) as usize, z);
                if ortho1 == MATERIAL_WALL_SOLID && ortho2 == MATERIAL_WALL_SOLID {
                    continue;
                }
            }

            let material = map.get_tile(nx, ny, z);
            if material == MATERIAL_WALL_SOLID {
                continue;
            }

            let base_cost: i16 = match material {
                MATERIAL_AIR => 10,
                MATERIAL_DOOR_WOOD => 40,
                MATERIAL_DOOR_IRON => 60,
                MATERIAL_DOOR_HEAVY => 110,
                MATERIAL_WALL_WOOD => 90,
                _ => 10,
            };

            let step_cost = if dx != 0 && dy != 0 {
                base_cost + 4 // diagonal cost ~1.4x
            } else {
                base_cost
            };

            let next_energy = energy - step_cost;
            if next_energy <= 0 {
                continue;
            }

            let next_idx = (n_ly as usize) * LOCAL_DIM + (n_lx as usize);
            if next_energy > energy_grid[next_idx] {
                energy_grid[next_idx] = next_energy;
                heap.push(State {
                    energy: next_energy,
                    x: nx,
                    y: ny,
                });
            }
        }
    }

    // Evaluate targets
    let mut results = Vec::with_capacity(targets.len());
    for &(tx, ty) in targets {
        if tx == src_x && ty == src_y {
            results.push(100);
            continue;
        }

        let dx_from_center = (tx as isize) - (src_x as isize);
        let dy_from_center = (ty as isize) - (src_y as isize);
        let lx = dx_from_center + LOCAL_CENTER;
        let ly = dy_from_center + LOCAL_CENTER;

        if lx < 0 || lx >= LOCAL_DIM as isize || ly < 0 || ly >= LOCAL_DIM as isize {
            results.push(0);
            continue;
        }

        let idx = (ly as usize) * LOCAL_DIM + (lx as usize);
        let rem_energy = energy_grid[idx];

        if rem_energy <= 0 {
            results.push(0);
        } else {
            let pct = ((rem_energy as i32 * 100) / (initial_energy as i32)).clamp(1, 100) as u8;
            results.push(pct);
        }
    }

    results
}

// ----------------------------------------------------------------------------
// BYOND FFI Exports
// ----------------------------------------------------------------------------

byond_fn! { fn acoustics_init(width_str, height_str, max_z_str) {
    let width = width_str.parse::<usize>().ok()?;
    let height = height_str.parse::<usize>().ok()?;
    let max_z = max_z_str.parse::<usize>().ok()?;

    let mut lock = MAP.write().ok()?;
    *lock = Some(AcousticMap::new(width, height, max_z));
    Some("OK".to_string())
} }

byond_fn! { fn acoustics_set_tile(x_str, y_str, z_str, mat_str) {
    let x = x_str.parse::<usize>().ok()?;
    let y = y_str.parse::<usize>().ok()?;
    let z = z_str.parse::<usize>().ok()?;
    let mat = mat_str.parse::<u8>().ok()?;

    let mut lock = MAP.write().ok()?;
    if let Some(map) = lock.as_mut() {
        map.set_tile(x, y, z, mat);
        Some("OK".to_string())
    } else {
        Some("ERR_NO_MAP".to_string())
    }
} }

byond_fn! { fn acoustics_query(z_str, src_x_str, src_y_str, voice_range_str, targets_str) {
    let z = z_str.parse::<usize>().ok()?;
    let src_x = src_x_str.parse::<usize>().ok()?;
    let src_y = src_y_str.parse::<usize>().ok()?;
    let voice_range = voice_range_str.parse::<usize>().ok()?;

    if targets_str.is_empty() {
        return Some("".to_string());
    }

    // Parse targets: "x1,y1;x2,y2;x3,y3"
    let mut targets = Vec::new();
    for pair in targets_str.split(';') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some((x_str, y_str)) = pair.split_once(',') {
            if let (Ok(x), Ok(y)) = (x_str.trim().parse::<usize>(), y_str.trim().parse::<usize>()) {
                targets.push((x, y));
            } else {
                targets.push((0, 0)); // Invalid coord -> 0% volume
            }
        }
    }

    let lock = MAP.read().ok()?;
    if let Some(map) = lock.as_ref() {
        let volumes = evaluate_acoustics(map, z, src_x, src_y, voice_range, &targets);
        let result = volumes
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(";");
        Some(result)
    } else {
        Some("".to_string())
    }
} }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_air_falloff() {
        let map = AcousticMap::new(50, 50, 1);
        let src = (25, 25);
        let voice_range = 7;
        let targets = vec![
            (25, 25), // Self: 100%
            (26, 25), // 1 tile east: 60/70 ~ 85%
            (28, 25), // 3 tiles east: 40/70 ~ 57%
            (31, 25), // 6 tiles east: 10/70 ~ 14%
            (32, 25), // 7 tiles east: 0%
            (35, 25), // 10 tiles east: 0%
        ];

        let results = evaluate_acoustics(&map, 1, src.0, src.1, voice_range, &targets);
        assert_eq!(results[0], 100);
        assert_eq!(results[1], 85);
        assert_eq!(results[2], 57);
        assert_eq!(results[3], 14);
        assert_eq!(results[4], 0);
        assert_eq!(results[5], 0);
    }

    #[test]
    fn test_solid_wall_blocks_sound() {
        let mut map = AcousticMap::new(50, 50, 1);
        // Build solid wall separating east and west halves completely
        for y in 1..=50 {
            map.set_tile(26, y, 1, MATERIAL_WALL_SOLID);
        }

        let src = (25, 25);
        let targets = vec![(27, 25)]; // right behind the wall

        // Normal voice
        let res_normal = evaluate_acoustics(&map, 1, src.0, src.1, 7, &targets);
        assert_eq!(res_normal[0], 0);

        // Shout
        let res_shout = evaluate_acoustics(&map, 1, src.0, src.1, 14, &targets);
        assert_eq!(res_shout[0], 0);
    }

    #[test]
    fn test_sound_bends_around_wall_opening() {
        let mut map = AcousticMap::new(50, 50, 1);
        // Wall from y=20 to y=30 at x=26, with opening at y=25
        for y in 20..=30 {
            if y != 25 {
                map.set_tile(26, y, 1, MATERIAL_WALL_SOLID);
            }
        }

        let src = (25, 25); // facing the open door
        let target = (27, 25); // on the other side of opening

        let res = evaluate_acoustics(&map, 1, src.0, src.1, 7, &[target]);
        // Sound passes through opening at (26, 25): 2 tiles total
        assert!(res[0] > 60);
    }

    #[test]
    fn test_closed_wooden_door_muffles_voice() {
        let mut map = AcousticMap::new(50, 50, 1);
        for y in 20..=30 {
            map.set_tile(26, y, 1, MATERIAL_WALL_SOLID);
        }
        // Wooden door at (26, 25)
        map.set_tile(26, 25, 1, MATERIAL_DOOR_WOOD);

        let src = (25, 25);
        let target = (27, 25); // listener right behind door

        // Whisper (range 1 -> 10 energy, door costs 40 -> inaudible)
        let res_whisper = evaluate_acoustics(&map, 1, src.0, src.1, 1, &[target]);
        assert_eq!(res_whisper[0], 0);

        // Normal voice (range 7 -> 70 energy, door costs 40 + air 10 = 50 cost -> 20/70 = 28%)
        let res_normal = evaluate_acoustics(&map, 1, src.0, src.1, 7, &[target]);
        assert_eq!(res_normal[0], 28);

        // Shout (range 14 -> 140 energy -> 90/140 = 64%)
        let res_shout = evaluate_acoustics(&map, 1, src.0, src.1, 14, &[target]);
        assert_eq!(res_shout[0], 64);
    }

    #[test]
    fn test_diagonal_wall_corner_leak_prevention() {
        let mut map = AcousticMap::new(50, 50, 1);
        // Place two solid walls touching corner-to-corner:
        // (26, 25) is SOLID
        // (25, 26) is SOLID
        map.set_tile(26, 25, 1, MATERIAL_WALL_SOLID);
        map.set_tile(25, 26, 1, MATERIAL_WALL_SOLID);

        let src = (25, 25);
        let target = (26, 26); // diagonally across the corner

        // Wall around corner blocks the diagonal slip
        // Let's also wall around to prevent walking around
        for x in 20..=30 {
            map.set_tile(x, 27, 1, MATERIAL_WALL_SOLID);
        }
        for y in 20..=30 {
            map.set_tile(27, y, 1, MATERIAL_WALL_SOLID);
        }

        let res = evaluate_acoustics(&map, 1, src.0, src.1, 7, &[target]);
        assert_eq!(res[0], 0);
    }
}

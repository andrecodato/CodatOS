//! Posição dos widgets numa grade de 12 colunas (RF-DASH-002).
//!
//! Compactação vertical, como nos dashboards conhecidos: não ficam buracos; um widget sobe
//! até encostar no de cima. Quem está sendo arrastado tem prioridade no lugar onde foi solto
//! e os outros se reorganizam em volta.

use serde::{Deserialize, Serialize};

pub const COLUMNS: u16 = 12;
/// Altura máxima de um widget, em linhas.
pub const MAX_ROWS: u16 = 12;

/// Retângulo em células da grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Cell {
    fn overlaps(self, other: Cell) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }

    /// Ajusta tamanho e posição para caber na grade.
    pub fn clamped(self, min_w: u16, min_h: u16) -> Cell {
        let w = self.w.clamp(min_w.max(1), COLUMNS);
        let h = self.h.clamp(min_h.max(1), MAX_ROWS);
        Cell {
            x: self.x.min(COLUMNS - w),
            y: self.y,
            w,
            h,
        }
    }

    /// Move `dx` colunas e `dy` linhas, sem sair da grade.
    pub fn shifted(self, dx: i32, dy: i32, min_w: u16, min_h: u16) -> Cell {
        Cell {
            x: offset(self.x, dx),
            y: offset(self.y, dy),
            ..self
        }
        .clamped(min_w, min_h)
    }

    /// Cresce ou encolhe pelo canto inferior direito; a posição não muda.
    pub fn resized(self, dw: i32, dh: i32, min_w: u16, min_h: u16) -> Cell {
        Cell {
            w: offset(self.w, dw).min(COLUMNS - self.x).max(min_w.max(1)),
            h: offset(self.h, dh).min(MAX_ROWS).max(min_h.max(1)),
            ..self
        }
    }
}

fn offset(value: u16, delta: i32) -> u16 {
    u16::try_from((i32::from(value) + delta).max(0)).unwrap_or(u16::MAX)
}

/// Compacta `cells` para cima. `priority` (o widget arrastado) ganha empates de linha.
/// A ordem dos itens no slice não muda.
pub fn compact(cells: &mut [Cell], priority: Option<usize>) {
    let mut order: Vec<usize> = (0..cells.len()).collect();
    order.sort_by_key(|&i| (cells[i].y, u8::from(Some(i) != priority), cells[i].x));
    let mut placed: Vec<Cell> = Vec::with_capacity(cells.len());
    for i in order {
        let mut cell = cells[i];
        // Sobe enquanto couber; se já colide onde está, desce até caber.
        while cell.y > 0
            && !collides(
                Cell {
                    y: cell.y - 1,
                    ..cell
                },
                &placed,
            )
        {
            cell.y -= 1;
        }
        while collides(cell, &placed) {
            cell.y += 1;
        }
        cells[i] = cell;
        placed.push(cell);
    }
}

fn collides(cell: Cell, placed: &[Cell]) -> bool {
    placed.iter().any(|p| p.overlaps(cell))
}

/// Primeira posição livre para um widget novo de `w × h`: a linha mais alta onde ele cabe.
pub fn free_spot(cells: &[Cell], w: u16, h: u16) -> Cell {
    let w = w.clamp(1, COLUMNS);
    let bottom = cells.iter().map(|c| c.y + c.h).max().unwrap_or(0);
    for y in 0..=bottom {
        for x in 0..=(COLUMNS - w) {
            let candidate = Cell { x, y, w, h };
            if !collides(candidate, cells) {
                return candidate;
            }
        }
    }
    Cell {
        x: 0,
        y: bottom,
        w,
        h,
    }
}

/// Total de linhas ocupadas.
pub fn rows(cells: &[Cell]) -> u16 {
    cells.iter().map(|c| c.y + c.h).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: u16, y: u16, w: u16, h: u16) -> Cell {
        Cell { x, y, w, h }
    }

    #[test]
    fn sobe_ate_encostar() {
        let mut cells = [c(0, 4, 3, 2), c(3, 9, 3, 2)];
        compact(&mut cells, None);
        assert_eq!(cells, [c(0, 0, 3, 2), c(3, 0, 3, 2)]);
    }

    #[test]
    fn arrastado_empurra_o_outro_para_baixo() {
        // A (0,0) arrastado para cima de B (0,2): ninguém sobrepõe, A fica no lugar.
        let mut cells = [c(0, 0, 4, 2), c(0, 2, 4, 2)];
        cells[1].y = 0; // B solto em cima de A
        compact(&mut cells, Some(1));
        assert_eq!(cells, [c(0, 2, 4, 2), c(0, 0, 4, 2)]);
    }

    #[test]
    fn arrastar_para_baixo_passa_por_baixo() {
        let mut cells = [c(0, 0, 4, 2), c(0, 2, 4, 2)];
        cells[0].y = 3; // A solto abaixo de B
        compact(&mut cells, Some(0));
        assert_eq!(cells, [c(0, 2, 4, 2), c(0, 0, 4, 2)]);
    }

    #[test]
    fn nunca_sobrepoe() {
        let mut cells = [c(0, 0, 6, 3), c(3, 1, 6, 2), c(5, 0, 7, 4), c(0, 0, 12, 1)];
        compact(&mut cells, Some(3));
        for (i, a) in cells.iter().enumerate() {
            for b in &cells[i + 1..] {
                assert!(!a.overlaps(*b), "{a:?} × {b:?}");
            }
        }
        assert_eq!(cells[3], c(0, 0, 12, 1));
    }

    #[test]
    fn lugar_livre_para_widget_novo() {
        let cells = [c(0, 0, 6, 2), c(6, 0, 3, 2)];
        assert_eq!(free_spot(&cells, 3, 2), c(9, 0, 3, 2));
        assert_eq!(free_spot(&cells, 4, 2), c(0, 2, 4, 2));
        assert_eq!(free_spot(&[], 3, 2), c(0, 0, 3, 2));
    }

    #[test]
    fn mover_e_redimensionar_respeitam_a_grade() {
        assert_eq!(c(2, 1, 3, 2).shifted(-5, -5, 1, 1), c(0, 0, 3, 2));
        assert_eq!(c(8, 0, 3, 2).shifted(4, 1, 1, 1), c(9, 1, 3, 2));
        assert_eq!(c(8, 0, 3, 2).resized(5, 20, 1, 1), c(8, 0, 4, 12));
        assert_eq!(c(0, 0, 3, 2).resized(-5, -5, 2, 2), c(0, 0, 2, 2));
    }

    #[test]
    fn limites_da_grade() {
        assert_eq!(c(10, 0, 4, 2).clamped(1, 1), c(8, 0, 4, 2));
        assert_eq!(c(0, 0, 1, 1).clamped(3, 2), c(0, 0, 3, 2));
        assert_eq!(c(0, 0, 20, 30).clamped(1, 1), c(0, 0, 12, 12));
    }
}

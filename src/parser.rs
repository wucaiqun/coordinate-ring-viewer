const RING_SEPARATOR: &str = "===============";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2D {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3D {
    pub lon: f64,
    pub lat: f64,
    pub alt: f64,
}

#[derive(Debug, Clone)]
pub struct Ring2D {
    pub points: Vec<Point2D>,
}

#[derive(Debug, Clone)]
pub struct Ring3D {
    pub points: Vec<Point3D>,
}

#[derive(Debug, Default, Clone)]
pub struct ParseResult2D {
    pub rings: Vec<Ring2D>,
    pub errors: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ParseResult3D {
    pub rings: Vec<Ring3D>,
    pub errors: Vec<String>,
}

pub fn parse_2d(input: &str) -> ParseResult2D {
    parse_rings(input, 2)
}

pub fn parse_3d(input: &str) -> ParseResult3D {
    let mut result = ParseResult3D::default();
    let blocks = split_rings(input);

    for (block_idx, block) in blocks.iter().enumerate() {
        let mut points = Vec::new();
        for (line_idx, line) in block.lines().enumerate() {
            let line = strip_comment(line).trim();
            if line.is_empty() {
                continue;
            }
            match parse_point_line(line, 3) {
                Ok([lon, lat, alt]) => points.push(Point3D { lon, lat, alt }),
                Err(msg) => result.errors.push(format!(
                    "Ring {} line {}: {}",
                    block_idx + 1,
                    line_idx + 1,
                    msg
                )),
            }
        }
        if !points.is_empty() {
            result.rings.push(Ring3D { points });
        }
    }

    result
}

fn parse_rings(input: &str, dims: usize) -> ParseResult2D {
    let mut result = ParseResult2D::default();
    let blocks = split_rings(input);

    for (block_idx, block) in blocks.iter().enumerate() {
        let mut points = Vec::new();
        for (line_idx, line) in block.lines().enumerate() {
            let line = strip_comment(line).trim();
            if line.is_empty() {
                continue;
            }
            match parse_point_line(line, dims) {
                Ok([lon, lat, _]) => points.push(Point2D { lon, lat }),
                Err(msg) => result.errors.push(format!(
                    "Ring {} line {}: {}",
                    block_idx + 1,
                    line_idx + 1,
                    msg
                )),
            }
        }
        if !points.is_empty() {
            result.rings.push(Ring2D { points });
        }
    }

    result
}

fn split_rings(input: &str) -> Vec<String> {
    input
        .split(RING_SEPARATOR)
        .map(str::trim)
        .filter(|block| !block.is_empty())
        .map(String::from)
        .collect()
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .or_else(|| line.split_once("//"))
        .map(|(before, _)| before)
        .unwrap_or(line)
}

fn parse_point_line(line: &str, dims: usize) -> Result<[f64; 3], &'static str> {
    let parts: Vec<f64> = line
        .split(|c: char| c == ',' || c.is_whitespace() || c == ';' || c == '\t')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    match (dims, parts.len()) {
        (2, 2) => Ok([parts[0], parts[1], 0.0]),
        (2, _) if parts.len() >= 2 => Ok([parts[0], parts[1], 0.0]),
        (3, 3) => Ok([parts[0], parts[1], parts[2]]),
        (3, _) if parts.len() >= 3 => Ok([parts[0], parts[1], parts[2]]),
        _ => Err("invalid coordinates; use 'lon lat' or 'lon lat altitude'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_rings_by_separator() {
        let input = "116.3 39.9\n117.0 40.0\n===============\n115.0 38.0\n115.5 38.5";
        let result = parse_2d(input);
        assert_eq!(result.rings.len(), 2);
        assert_eq!(result.rings[0].points.len(), 2);
        assert_eq!(result.rings[1].points.len(), 2);
    }

    #[test]
    fn parses_3d_with_altitude() {
        let input = "116.3 39.9 100\n117.0 40.0 200";
        let result = parse_3d(input);
        assert_eq!(result.rings.len(), 1);
        assert_eq!(result.rings[0].points[0].alt, 100.0);
    }
}

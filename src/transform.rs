#[derive(Debug,Clone)]
pub struct Matrix {
    values: Vec<f64>,
}

impl Matrix {
    pub fn new() -> Matrix {
        return Matrix {
            values: vec![
                1.0, 0.0, 0.0,
                0.0, 1.0, 0.0,
                0.0, 0.0, 1.0
            ],
        };
    }

    pub fn vec6_to_matrix(v: &Vec<f64>) -> Matrix {
        assert_eq!(v.len(), 6);
        let result = vec![
            v[0], v[1], 0.0,
            v[2], v[3], 0.0,
            v[4], v[5], 1.0,
        ];
        return Matrix::from(result);
    }

    pub fn x_scale(&self) -> f64 { self.values[0] }
    pub fn y_scale(&self) -> f64 { self.values[4] }
    pub fn x(&self) -> f64 { self.values[6] }
    pub fn y(&self) -> f64 { self.values[7] }

    pub fn set_x(&mut self, value: f64) {
        self.values[6] = value;
    }

    pub fn from(vec: Vec<f64>) -> Matrix {
        assert_eq!(vec.len(), 9);
        return Matrix {
            values: vec,
        };
    }
}

pub fn multiply_3d(m1: &Matrix, m2: &Matrix) -> Matrix {
    let v1 = &m1.values;
    let v2 = &m2.values;

    let mut result = vec![0.0; 9];
    assert_eq!(v1.len(), 9);
    assert_eq!(v2.len(), 9);

    for row in 0..3 {
        for col in 0..3 {
            for k in 0..3 {
                result[row*3 + col] += v1[row*3 + k] * v2[k*3 + col];
            }
        }
    }
    return Matrix::from(result);
}


/*
 *  We will work in the category FinSet...
 *      - A `Function` is then simply an array of natural numbers.
 */
pub type Value = usize;

#[derive(Debug)]
pub struct Function {
    data: Vec<Option<Value>>
}

impl Function {
    pub fn new<T: IntoIterator<Item = usize>>(data: T) -> Function {
        Function {
            data: data.into_iter()
                .map(|value| Some(value))
                .collect()
        }
    }

    pub fn domain(&self) -> usize {
        self.data.len()
    }

    pub fn range(&self) -> Option<usize> {
        self.data.iter()
            .copied()
            .max()
            .flatten()
    }

    pub fn apply(&self, value: Value) -> Option<Value> {
        self.data.get(value)
            .copied()
            .flatten()
    }

    pub fn identity(size: usize) -> Function {
        Function {
            data: (0..size)
                .map(|x| Some(x))
                .collect()
        }
    }

    pub fn compose(&self, other: &Function) -> Function {
        let mut function = Function { 
            data: Vec::new()
        };
        for elem in self.data.iter() {
            if let Some(output) = elem {
                function.data.push(other.apply(*output));
            }
        }
        function
    }

    pub fn pullback(&self, other: &Function) -> (Function, Function) {
        let mut down = Function {
            data: Vec::new()
        };
        let mut across = Function {
            data: Vec::new()
        };
        for (i, x) in self.data.iter().copied().enumerate() {
            for (j, y) in other.data.iter().copied().enumerate() {
                if x == y {
                    down.data.push(Some(i));
                    across.data.push(Some(j));
                } else {
                    down.data.push(None);
                    across.data.push(None);
                }
            }
        }
        (down, across)
    }

    pub fn pushout(&self, other: &Function) -> (Function, Function) {
        let mut down = Function {
            data: Vec::new()
        };
        let mut across = Function {
            data: Vec::new()
        };

        (down, across)
    }
}

impl<T> From<T> for Function where T: IntoIterator<Item = usize> {
    fn from(value: T) -> Self {
        Function::new(value)
    }
}


#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Arrow<E, T> {
    source: E,
    target: E,
    data: T
}

pub struct Rule<E, T> {
    left: Arrow<E, T>,
    right: Arrow<E, T>,
    composition: Arrow<E, T>
}

pub struct Category<E, T> {
    objects: Vec<E>,
    arrows: Vec<Arrow<E, T>>,
    rules: Vec<Rule<E, T>>
}

impl<E, T> Category<E, T> where E: Eq {
    fn source_arrows<'a>(&'a self, object: &'a E) -> impl Iterator<Item = &Arrow<E, T>> {
        self.arrows
            .iter()
            .filter(|arrow| arrow.source == *object)
    }
    
    fn target_arrows<'a>(&'a self, object: &'a E) -> impl Iterator<Item = &Arrow<E, T>> {
        self.arrows
            .iter()
            .filter(|arrow| arrow.target == *object)
    }

    pub fn is_consistent(&self) -> bool {
        for arrow in &self.arrows {
            if !self.objects.contains(&arrow.source) ||
               !self.objects.contains(&arrow.target)
            {
                return false
            }


        }
        for rule in &self.rules {
            if rule.left.target        == rule.right.source &&
               rule.composition.source == rule.left.source &&
               rule.composition.target == rule.right.target
            {
                return false
            }
        }
        for object in &self.objects {
            for arrow in &self.arrows {
                if &arrow.source == object && &arrow.target == object {
                    for rule in &self.rules {

                    }
                }
            }
        }
        
        true
    }
}
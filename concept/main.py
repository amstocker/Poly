# In this algebra, a "Function" is either a list of pairs (a, b) where each 'a' is unique




class Function:
  def __init__(self, data, depth=1):
    if type(data) == Function:
      return data
    else:
      self.data = set(data)
      self.depth = depth

  def domain(self):
    return {a for (a, _) in self.data}

  def codomain(self):
    return {Function(self.eval(a), depth=self.depth-1) for a in self.domain()}

  def eval(self, x):
    return Function(
      set(map(
        lambda t: t[1],
        filter(
          lambda t: t[0]==x,
          self.data
        )
      )),
      depth=self.depth - 1
    )
    
  def compose(self, other):
    if not self.depth == other.depth:
      return EMPTY
    
  def __repr__(self) -> str:
    return str(self.data)


EMPTY = Function(set(), depth=0)


TEST_FUNC = Function([(1,1), (2,3), (3, 2)])

TEST_LENS = Function([
  (1, (5, 5)), (1, (6, 6)),
  (2, (5, 6)), (2, (6, 5))
], depth=2)

print(TEST_FUNC.domain())
print(TEST_FUNC.codomain())
print(TEST_LENS.codomain())

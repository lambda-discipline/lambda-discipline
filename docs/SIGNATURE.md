## Builtin's Signature

```haskell
print: Any... -> Void
add: Float -> Float -> Float
sub: Float -> Float -> Float
mul: Float -> Float -> Float
cmp: Float -> Float -> Float
if_zero: Float -> A -> B -> A|B
sort: List<Float> -> List<Float>
parse_float: String -> Float

-- List module
list: Any... -> List
list_size: List -> Float
list_get: List -> Float -> Any
list_remove: List -> Float -> Any
list_append: List -> Any -> Void
map: (A -> B) -> List<A> -> List<B>
transpose: List<List> -> List<List>
zip: List<List> -> List<List>
foldr: (A -> B -> B) -> List<A> -> B -> B
 
-- IO module
slurp_file: String -> String
lines: String -> List<String>
split: String -> String -> List<String>
```

> TODO: Add Alias for Path as String, etc.
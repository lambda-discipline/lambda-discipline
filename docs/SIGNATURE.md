## Builtin's Signature

```haskell
print: Any... -> Void
add: Float -> Float -> Float
sub: Float -> Float -> Float
mul: Float -> Float -> Float
cmp: Float -> Float -> Float
null: Float -> A -> B -> A|B
sort: List<Float> -> List<Float>
parseFloat: String -> Float

-- List module
list: Any... -> List
listSize: List -> Float
listGet: List -> Float -> Any
listRemove: List -> Float -> Any
listAppend: List -> Any -> Void
map: (A -> B) -> List<A> -> List<B>
transpose: List<List> -> List<List>
zip: List<List> -> List<List>
foldRight: (A -> B -> B) -> List<A> -> B -> B
 
-- IO module
slurpFile: String -> String
lines: String -> List<String>
split: String -> String -> List<String>
```

> TODO: Add Alias for Path as String, etc.
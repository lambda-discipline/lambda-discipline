#pragma once

namespace lbd::type
{
  enum class TypeKind
  {
    NAMED, // Number, String
    VARIABLE, // a, b
    APPLIED, // List<Number>, Map<String, Number>
    FUNCTION, // String -> Number
    QUALIFIED, // Eq a => a -> a
    ANY,
  };
}
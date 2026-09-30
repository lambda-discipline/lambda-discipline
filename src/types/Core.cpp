#include <functional>
#include <variant>
#include <lbd/runtime/Interpreter.hpp>
#include <lbd/types/Core.hpp>

namespace lbd::types
{
  static size_t hashCombine(const size_t a, const size_t b)
  {
    // ReSharper disable once CppRedundantParentheses
    return a ^ (b + 0x9e3779b9 + (a << 6) + (a >> 2));
  }

  size_t TypePtrHash::operator()(const TypePtr &t) const noexcept { return t->hash(); }

  bool TypePtrEq::operator()(const TypePtr &a, const TypePtr &b) const noexcept { return *a == *b; }

  bool Constraint::operator==(const Constraint &other) const noexcept
  {
    return className == other.className && type->equals(*other.type);
  }

  TypeTag typeTagFromValue(const runtime::Value &value);

  TypePtr Type::named(const std::string &name)
  {
    const auto type = std::make_shared<Type>();
    type->m_kind = TypeKind::Named;
    type->m_name = name;
    return type;
  }

  TypePtr Type::variable(const std::string &name)
  {
    const auto type = std::make_shared<Type>();
    type->m_kind = TypeKind::Variable;
    type->m_name = name;
    return type;
  }

  TypePtr Type::applied(const TypePtr &base, std::vector<TypePtr> arguments)
  {
    auto type = std::make_shared<Type>();
    type->m_kind = TypeKind::Applied;
    type->m_base = base;
    type->m_arguments = std::move(arguments);
    return type;
  }

  TypePtr Type::function(const TypePtr &from, const TypePtr &to)
  {
    auto type = std::make_shared<Type>();
    type->m_kind = TypeKind::Function;
    type->m_from = from;
    type->m_to = to;
    return type;
  }

  TypePtr Type::qualified(std::vector<Constraint> constraints, const TypePtr &qualifiedType)
  {
    auto type = std::make_shared<Type>();
    type->m_kind = TypeKind::Qualified;
    type->m_constraints = std::move(constraints);
    type->m_qualifiedType = qualifiedType;
    return type;
  }

  TypePtr Type::any()
  {
    auto type = std::make_shared<Type>();
    type->m_kind = TypeKind::Any;
    return type;
  }

  TypePtr Type::closure()
  {
    auto type = std::make_shared<Type>();
    type->m_kind = TypeKind::Closure;
    return type;
  }

  TypePtr Type::nativeFunction()
  {
    auto type = std::make_shared<Type>();
    type->m_kind = TypeKind::NativeFunction;
    return type;
  }

  TypeKind Type::getKind() const noexcept { return m_kind; }
  const std::string &Type::getName() const noexcept { return m_name; }
  const TypePtr &Type::getBase() const noexcept { return m_base; }
  const std::vector<TypePtr> &Type::getArguments() const noexcept { return m_arguments; }
  const TypePtr &Type::getFrom() const noexcept { return m_from; }
  const TypePtr &Type::getTo() const noexcept { return m_to; }
  const std::vector<Constraint> &Type::getConstraints() const noexcept { return m_constraints; }
  const TypePtr &Type::getQualifiedType() const noexcept { return m_qualifiedType; }
  bool Type::allowsHardCheck() const noexcept { return m_hardCheck; }
  void Type::setAllowsHardCheck(const bool check) noexcept { m_hardCheck = check; }

  bool Type::equals(const Type &other) const noexcept
  {
    if (m_kind != other.m_kind) return false;

    switch (m_kind)
    {
      case TypeKind::Any: return true;

      case TypeKind::Named:
      case TypeKind::Variable: return m_name == other.m_name;

      case TypeKind::Applied:
      {
        if (!m_base->equals(*other.m_base) || m_arguments.size() != other.m_arguments.size())
          return false;
        for (size_t i = 0; i < m_arguments.size(); ++i)
          if (!m_arguments[i]->equals(*other.m_arguments[i])) return false;
        return true;
      }

      case TypeKind::Function: return m_from->equals(*other.m_from) && m_to->equals(*other.m_to);

      case TypeKind::Qualified:
        if (m_constraints.size() != other.m_constraints.size()) return false;
        for (size_t i = 0; i < m_constraints.size(); ++i)
          if (m_constraints[i] != other.m_constraints[i]) return false;
        return m_qualifiedType->equals(*other.m_qualifiedType);

      case TypeKind::Closure:
      case TypeKind::NativeFunction: return true;
    }
    return false;
  }

  bool Type::accepts(const Type &other) const noexcept
  {
    if (m_kind == TypeKind::Any) return true;
    if (m_kind != other.m_kind) return false;

    switch (m_kind)
    {
      case TypeKind::Named: return m_name == other.m_name;

      case TypeKind::Variable: return true;

      case TypeKind::Applied:
      {
        if (!m_base->accepts(*other.m_base) || m_arguments.size() != other.m_arguments.size())
          return false;
        for (size_t i = 0; i < m_arguments.size(); ++i)
          if (!m_arguments[i]->accepts(*other.m_arguments[i])) return false;
        return true;
      }

      case TypeKind::Function:
        return other.m_from->accepts(*m_from) && m_to->accepts(*other.m_to);

      case TypeKind::Qualified:
        return m_qualifiedType->accepts(*other.m_qualifiedType);

      case TypeKind::Closure:
      case TypeKind::NativeFunction: return true;

      case TypeKind::Any: return true;
    }

    return false;
  }

  bool Type::operator==(const Type &other) const noexcept { return equals(other); }

  size_t Type::hash() const noexcept
  {
    size_t hash = std::hash<int>{}(static_cast<int>(m_kind));

    switch (m_kind)
    {
      case TypeKind::Any:
      case TypeKind::Closure:
      case TypeKind::NativeFunction: return hash;

      case TypeKind::Named:
      case TypeKind::Variable: return hashCombine(hash, std::hash<std::string>{}(m_name));

      case TypeKind::Applied:
      {
        hash = hashCombine(hash, m_base->hash());
        for (const auto &argument: m_arguments)
          hash = hashCombine(hash, argument->hash());
        return hash;
      }

      case TypeKind::Function:
        hash = hashCombine(hash, m_from->hash());
        hash = hashCombine(hash, m_to->hash());
        return hash;

      case TypeKind::Qualified:
      {
        for (const auto &[className, type]: m_constraints)
        {
          hash = hashCombine(hash, std::hash<std::string>{}(className));
          hash = hashCombine(hash, type->hash());
        }
        hash = hashCombine(hash, m_qualifiedType->hash());
        return hash;
      }
    }

    return hash;
  }

  std::string typeTagToString(const TypeTag tag) noexcept
  {
    switch (tag)
    {
      case TypeTag::Float: return "Float";
      case TypeTag::String: return "String";
      case TypeTag::Closure: return "Closure";
      case TypeTag::NativeFunction: return "NativeFunction";
      case TypeTag::List: return "List";
      case TypeTag::Any: return "Any";
      default: return "UnknownType";
    }
  }

  TypeTag typeTagFromValue(const runtime::Value &value)
  {
    return std::visit([&]<typename T0>(T0 &&) -> TypeTag
    {
      using T = std::decay_t<T0>;
      if constexpr (std::is_same_v<T, double>)
      {
        return TypeTag::Float;
      } else if constexpr (std::is_same_v<T, std::string>)
      {
        return TypeTag::String;
      } else if constexpr (std::is_same_v<T, runtime::Closure>)
      {
        return TypeTag::Closure;
      } else if constexpr (std::is_same_v<T, std::shared_ptr<runtime::NativeFunction>>)
      {
        return TypeTag::NativeFunction;
      } else if constexpr (std::is_same_v<T, std::shared_ptr<runtime::List>>)
      {
        return TypeTag::List;
      } else
      {
        return TypeTag::Any;
      }
    }, value);
  }

  TypeTag Type::typeTag() const noexcept
  {
    if (m_tagCached) return m_cachedTag;

    switch (m_kind)
    {
      case TypeKind::Named:
        if (m_name == "Float" || m_name == "Number") m_cachedTag = TypeTag::Float;
        else if (m_name == "String") m_cachedTag = TypeTag::String;
        else m_cachedTag = TypeTag::Any;
        break;

      case TypeKind::Applied:
        if (m_base && m_base->getKind() == TypeKind::Named && m_base->getName() == "List")
          m_cachedTag = TypeTag::List;
        else
          m_cachedTag = TypeTag::Any;
        break;

      case TypeKind::Closure: m_cachedTag = TypeTag::Closure;
        break;

      case TypeKind::NativeFunction: m_cachedTag = TypeTag::NativeFunction;
        break;

      case TypeKind::Any:
      case TypeKind::Function:
      case TypeKind::Variable:
      case TypeKind::Qualified: m_cachedTag = TypeTag::Any;
        break;
    }

    m_tagCached = true;
    return m_cachedTag;
  }

  bool Type::matches(const runtime::Value &value) const noexcept
  {
    return typeTag() == TypeTag::Any || typeTag() == typeTagFromValue(value);
  }

  std::ostream &operator<<(std::ostream &outputStream, const Type &type) noexcept
  {
    switch (type.m_kind)
    {
      case TypeKind::Any:
        outputStream << "Any";
        break;

      case TypeKind::Named:
      case TypeKind::Variable:
        outputStream << type.m_name;
        break;

      case TypeKind::Applied:
      {
        outputStream << *type.m_base << "<";
        for (size_t i = 0; i < type.m_arguments.size(); ++i)
        {
          if (i) outputStream << ", ";

          if (type.m_arguments[i]->getKind() == TypeKind::Function)
            outputStream << "(" << *type.m_arguments[i] << ")";
          else
            outputStream << *type.m_arguments[i];
        }
        outputStream << ">";
        break;
      }

      case TypeKind::Function:
      {
        if (type.m_from->getKind() == TypeKind::Function)
          outputStream << "(" << *type.m_from << ")";
        else
          outputStream << *type.m_from;

        outputStream << " -> ";
        outputStream << *type.m_to;
        break;
      }

      case TypeKind::Qualified:
      {
        for (size_t i = 0; i < type.m_constraints.size(); ++i)
        {
          if (i) outputStream << ", ";
          outputStream << type.m_constraints[i].className << " " << *type.m_constraints[i].type;
        }
        outputStream << " => " << *type.m_qualifiedType;
        break;
      }

      case TypeKind::Closure:
        outputStream << "Closure";
        break;

      case TypeKind::NativeFunction:
        outputStream << "NativeFunction";
        break;
    }

    return outputStream;
  }
}

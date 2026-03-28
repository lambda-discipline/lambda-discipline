#include <functional>
#include <lbd/type/Type.hpp>

namespace lbd::type
{
    static size_t hashCombine(const size_t a, const size_t b)
    {
        // ReSharper disable once CppRedundantParentheses
        return a ^ (b + 0x9e3779b9 + (a << 6) + (a >> 2));
    }

    size_t TypePtrHash::operator()(const TypePtr& t) const noexcept { return t->hash(); }

    bool TypePtrEq::operator()(const TypePtr& a, const TypePtr& b) const noexcept { return *a == *b; }

    bool Constraint::operator==(const Constraint& other) const noexcept
    {
        return className == other.className && type->equals(*other.type);
    }

    TypePtr Type::named(const std::string& name)
    {
        const auto type = std::make_shared<Type>();
        type->kind = TypeKind::NAMED;
        type->name = name;
        return type;
    }

    TypePtr Type::variable(const std::string& name)
    {
        const auto type = std::make_shared<Type>();
        type->kind = TypeKind::VARIABLE;
        type->name = name;
        return type;
    }

    TypePtr Type::applied(const TypePtr& base, std::vector<TypePtr> arguments)
    {
        auto type = std::make_shared<Type>();
        type->kind = TypeKind::APPLIED;
        type->base = base; // Most likely would be a NAMED: List, Map.
        type->arguments = std::move(arguments);
        return type;
    }

    TypePtr Type::function(const TypePtr& from, const TypePtr& to)
    {
        auto type = std::make_shared<Type>();
        type->kind = TypeKind::FUNCTION;
        type->from = from;
        type->to = to;
        return type;
    }

    TypePtr Type::qualified(std::vector<Constraint> constraints, const TypePtr& qualifiedType)
    {
        auto type = std::make_shared<Type>();
        type->kind = TypeKind::QUALIFIED;
        type->constraints = std::move(constraints);
        type->qualifiedType = qualifiedType;
        return type;
    }

    TypePtr Type::any()
    {
        auto type = std::make_shared<Type>();
        type->kind = TypeKind::ANY;
        return type;
    }

    TypeKind Type::getKind() const noexcept { return kind; }
    const std::string& Type::getName() const noexcept { return name; }
    const TypePtr& Type::getBase() const noexcept { return base; }
    const std::vector<TypePtr>& Type::getArguments() const noexcept { return arguments; }
    const TypePtr& Type::getFrom() const noexcept { return from; }
    const TypePtr& Type::getTo() const noexcept { return to; }

    const std::vector<Constraint>& Type::getConstraints() const noexcept { return constraints; }

    const TypePtr& Type::getQualifiedType() const noexcept { return qualifiedType; }

    bool Type::equals(const Type& other) const noexcept
    {
        if (kind != other.kind) return false;

        switch (kind)
        {
            case TypeKind::ANY: return true;

            case TypeKind::NAMED:
            case TypeKind::VARIABLE: return name == other.name;

            case TypeKind::APPLIED:
                {
                    if (!base->equals(*other.base) || arguments.size() != other.arguments.size())
                        return false;
                    for (size_t i = 0; i < arguments.size(); ++i)
                        if (!arguments[i]->equals(*other.arguments[i])) return false;
                    return true;
                }

            case TypeKind::FUNCTION: return from->equals(*other.from) && to->equals(*other.to);

            case TypeKind::QUALIFIED:
                if (constraints.size() != other.constraints.size()) return false;
                for (size_t i = 0; i < constraints.size(); ++i)
                    if (constraints[i] != other.constraints[i]) return false;
                return qualifiedType->equals(*other.qualifiedType);
        }
        return false;
    }

    bool Type::accepts(const Type& other) const noexcept
    {
        // Any accepts everything
        if (kind == TypeKind::ANY) return true;
        if (kind != other.kind) return false;

        switch (kind)
        {
            case TypeKind::NAMED: return name == other.name;

            case TypeKind::VARIABLE:
                // Type variables accept any type (like generics).
                return true;

            case TypeKind::APPLIED:
                {
                    if (!base->accepts(*other.base) || arguments.size() != other.arguments.size()) return false;
                    for (size_t i = 0; i < arguments.size(); ++i)
                        if (!arguments[i]->accepts(*other.arguments[i])) return false;
                    return true;
                }

            case TypeKind::FUNCTION:
                // Function types are CONTRAVARIANT in arguments and COVARIANT in return type.
                return other.from->accepts(*from) && // contra-variant
                    to->accepts(*other.to); // co-variant

            case TypeKind::QUALIFIED:
                // Ignore constraints for now, just check underlying type.
                return qualifiedType->accepts(*other.qualifiedType);

            case TypeKind::ANY: return true;
        }

        return false;
    }

    bool Type::operator==(const Type& other) const noexcept { return equals(other); }

    size_t Type::hash() const noexcept
    {
        size_t hash = std::hash<int>{}(static_cast<int>(kind));

        switch (kind)
        {
            case TypeKind::ANY: return hash;

            case TypeKind::NAMED:
            case TypeKind::VARIABLE: return hashCombine(hash, std::hash<std::string>{}(name));

            case TypeKind::APPLIED:
                {
                    hash = hashCombine(hash, base->hash());
                    for (const auto& argument : arguments)
                        hash = hashCombine(hash, argument->hash());
                    return hash;
                }

            case TypeKind::FUNCTION:
                hash = hashCombine(hash, from->hash());
                hash = hashCombine(hash, to->hash());
                return hash;

            case TypeKind::QUALIFIED:
                {
                    for (const auto& [className, type] : constraints)
                    {
                        hash = hashCombine(hash, std::hash<std::string>{}(className));
                        hash = hashCombine(hash, type->hash());
                    }
                    hash = hashCombine(hash, qualifiedType->hash());
                    return hash;
                }
        }

        return hash;
    }

    std::ostream& operator<<(std::ostream& os, const Type& type) noexcept
    {
        switch (type.kind)
        {
            case TypeKind::ANY:
                os << "Any";
                break;

            case TypeKind::NAMED:
            case TypeKind::VARIABLE:
                os << type.name;
                break;

            case TypeKind::APPLIED:
                {
                    os << *type.base << "<";
                    for (size_t i = 0; i < type.arguments.size(); ++i)
                    {
                        if (i) os << ", ";

                        // Parenthesize if the argument is a function.
                        if (type.from->getKind() == TypeKind::FUNCTION)
                            os << "(" << *type.arguments[i] << ")";
                        else
                            os << *type.arguments[i];
                    }
                    os << ">";
                    break;
                }

            case TypeKind::FUNCTION:
                {
                    // Parenthesize left side if it's also a function.
                    if (type.from->getKind() == TypeKind::FUNCTION)
                        os << "(" << *type.from << ")";
                    else
                        os << *type.from;

                    os << " -> ";
                    os << *type.to;
                    break;
                }

            case TypeKind::QUALIFIED:
                {
                    for (size_t i = 0; i < type.constraints.size(); ++i)
                    {
                        if (i) os << ", ";
                        os << type.constraints[i].className << " " << *type.constraints[i].type;
                    }
                    os << " => " << *type.qualifiedType;
                    break;
                }
        }

        return os;
    }
}

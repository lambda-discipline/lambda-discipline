#include <lbd/frontend/location.h>
#include <lbd/frontend/lexer.h>
#include <lbd/frontend/parser.h>
#include <lbd/utils/string_escape.h>
#include <unordered_set>
#include <filesystem>

namespace frontend {
    static std::unordered_set<std::string> loadedFiles;

    Parser::Parser(const std::vector<token::Token> &tokens,
                   const context::Options options) : options(options), tokens(tokens), index(0) {
    }

    Program Parser::parse() {
        return Program{build()};
    }

    template<typename T>
    void Parser::assertToken() {
        if (const token::Token &currentToken = tokens[index]; !std::holds_alternative<T>(currentToken.tokenType)) {
            options.logger.error(currentToken.location, "syntax error: expected ", token::toString<T>(), ", got ",
                                 currentToken.toString());
        }
    }

    template<typename T>
    void Parser::assertAndConsume() {
        assertToken<T>();
        ++index;
    }

    IdentifierAstNode Parser::consumeIdentifier() {
        assertToken<token::Identifier>();
        auto [value] = std::get<token::Identifier>(tokens[index].tokenType);
        const Location location = tokens[index].location;
        ++index;
        return IdentifierAstNode{value, location};
    }

    interpreter::type::PrimitiveType
    Parser::consumePrimitiveTypeName() {
        assertToken<token::Identifier>();
        auto [value] = std::get<token::Identifier>(tokens[index].tokenType);
        Location location = tokens[index].location;
        ++index;
        // TODO: Perform this check through interpreter::type package.
        if (value == "Float") {
            return interpreter::type::PrimitiveType{interpreter::type::PrimitiveType::Type::Float};
        }
        if (value == "Str") {
            return interpreter::type::PrimitiveType{interpreter::type::PrimitiveType::Type::String};
        }
        if (value == "Any") {
            return interpreter::type::PrimitiveType{interpreter::type::PrimitiveType::Type::Any};
        }
        return interpreter::type::PrimitiveType{interpreter::type::PrimitiveType::Type::Custom, std::move(value)};
    }

    interpreter::type::Type Parser::parseType() {
        interpreter::type::PrimitiveType typ = consumePrimitiveTypeName();
        std::vector types{typ};
        while (std::holds_alternative<token::Arrow>(tokens[index].tokenType)) {
            assertAndConsume<token::Arrow>();
            types.push_back(consumePrimitiveTypeName());
        }
        if (types.size() == 1) {
            return typ;
        }
        // Construct Right Associative AST
        interpreter::type::Type currentType = types.back();
        for (int typeIndex = static_cast<int>(types.size()) - 2; typeIndex >= 0; --typeIndex) {
            auto nextCompoundType = std::make_shared<interpreter::type::CompoundType>();
            nextCompoundType->leftType = types[typeIndex];
            nextCompoundType->rightType = currentType;
            currentType = nextCompoundType;
        }
        return currentType;
    }

    Expression Parser::parseExpression() {
        const auto &token = tokens[index];
        const Location location = token.location;
        return std::visit([&]<typename T0>(T0 &&) {
            using T = std::decay_t<T0>;
            if (std::is_same_v<T, token::Identifier>) {
                ++index;
                const auto value = std::get<token::Identifier>(token.tokenType).value;
                return Expression(IdentifierAstNode{value, location});
            }
            if (std::is_same_v<T, token::String>) {
                ++index;
                const auto value = unescapeString(std::get<token::String>(token.tokenType).value);
                return Expression(StringAstNode{value, location});
            }
            if (std::is_same_v<T, token::Float>) {
                ++index;
                const auto value = std::get<token::Float>(token.tokenType).value;
                return Expression(FloatAstNode{value, location});
            }
            if (std::is_same_v<T, token::BackwardSlash>) {
                return Expression(parseLambdaExpression());
            }
            if (std::is_same_v<T, token::OpenParenthesis>) {
                return Expression(parseFunctionApplication());
            }
            options.logger.error(location, "syntax error: unexpected token ", token.toString());
        }, token.tokenType);
    }

    LambdaExpression Parser::parseLambdaExpression() {
        Location location = tokens[index].location;
        assertAndConsume<token::BackwardSlash>();
        IdentifierAstNode argument = consumeIdentifier();
        assertAndConsume<token::Colon>();
        interpreter::type::Type argumentType = parseType();
        assertAndConsume<token::Dot>();
        Expression expression = parseExpression();
        return LambdaExpression{
            argument, argumentType, std::make_unique<Expression>(std::move(expression)), location
        };
    }

    FunctionApplication Parser::parseFunctionApplication() {
        const Location location = tokens[index].location;
        assertAndConsume<token::OpenParenthesis>();
        const IdentifierAstNode functionName = consumeIdentifier();
        std::vector<std::unique_ptr<Expression> > arguments;
        while (!std::holds_alternative<token::CloseParenthesis>(tokens[index].tokenType)) {
            arguments.push_back(std::make_unique<Expression>(std::move(parseExpression())));
        }
        assertAndConsume<token::CloseParenthesis>();
        return FunctionApplication{functionName, std::move(arguments), location};
    }

    DefinitionAstNode Parser::parseDefinitionAstNode() {
        Location location = tokens[index].location;
        IdentifierAstNode definitionName = consumeIdentifier();
        assertAndConsume<token::Colon>();
        interpreter::type::Type definitionType = parseType();
        assertAndConsume<token::Equal>();
        Expression expression = parseExpression();
        return DefinitionAstNode{definitionName, definitionType, std::move(expression), location};
    }

    static std::string getAbsolutePath(const std::string &filepath) {
        return std::filesystem::absolute(filepath).string();
    }

    static void processUseFile(std::vector<AstNode> &astNodes, const std::string &filepath,
                               const context::Options &options) {
        const std::string absolutePath = getAbsolutePath(filepath);
        if (loadedFiles.contains(absolutePath)) {
            // Circular Dependency or Duplicate Load
            return;
        }
        loadedFiles.insert(absolutePath);
        auto lexerValue = Lexer::fromFile(filepath, options);
        const auto tokens = lexerValue.lex();
        if (options.debug) {
            for (const auto &token: tokens) {
                options.logger.debug(token);
            }
        }
        for (auto [astastNodes] = Parser(tokens, options).parse(); auto &astNode: astastNodes) {
            astNodes.emplace_back(std::move(astNode));
        }
    }

    std::vector<AstNode> Parser::build() {
        std::vector<AstNode> astNodes;
        while (!std::holds_alternative<token::Eof>(tokens[index].tokenType)) {
            const token::Token &token = tokens[index];
            std::visit([&]<typename T0>(T0 &&) {
                using T = std::decay_t<T0>;
                if constexpr (std::is_same_v<T, token::Identifier>) {
                    if (const auto identifierValue = std::get<token::Identifier>(tokens[index].tokenType).value;
                        identifierValue == "use") {
                        ++index; // consume "use"
                        assertToken<token::String>();
                        const std::string filepath = unescapeString(
                            std::get<token::String>(tokens[index].tokenType).value);
                        ++index; // consume <filepath>
                        processUseFile(astNodes, filepath, options);
                    } else {
                        astNodes.push_back(AstNode{std::move(parseDefinitionAstNode())});
                    }
                } else if constexpr (std::is_same_v<T, token::String> ||
                                     std::is_same_v<T, token::Float> ||
                                     std::is_same_v<T, token::BackwardSlash> ||
                                     std::is_same_v<T, token::OpenParenthesis>) {
                    astNodes.push_back(AstNode{std::move(parseExpression())});
                } else {
                    options.logger.error(token.location, "syntax error: unexpected token ", token.toString());
                }
            }, token.tokenType);
        }
        return astNodes;
    }
}

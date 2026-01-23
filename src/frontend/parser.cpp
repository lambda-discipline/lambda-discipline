#include <lbd/frontend/location.h>
#include <lbd/frontend/lexer.h>
#include <lbd/frontend/parser.h>
#include <lbd/utils/string_escape.h>
#include <unordered_set>
#include <filesystem>

namespace frontend {
    static std::unordered_set<std::string> loadedFiles;
    static context::Options optionsValue;

    Parser::Parser(const std::vector<token::Token> &tokens, const context::Options options_) {
        optionsValue = options_;
        size_t index = 0;
        program = Program{build(tokens, index)};
    }

    template<typename T>
    void Parser::assertToken(const std::vector<token::Token> &tokens, size_t &index) {
        if (const token::Token &currentToken = tokens[index]; !std::holds_alternative<T>(currentToken.tokenType)) {
            optionsValue.logger.error(currentToken.location, "syntax error: expected ", token::toString<T>(), ", got ",
                                      currentToken.toString());
        }
    }

    template<typename T>
    void Parser::assertAndConsume(const std::vector<token::Token> &tokens, size_t &index) {
        assertToken<T>(tokens, index);
        ++index;
    }

    IdentifierAstNode Parser::consumeIdentifier(const std::vector<token::Token> &tokens, size_t &index) {
        assertToken<token::Identifier>(tokens, index);
        auto [value] = std::get<token::Identifier>(tokens[index].tokenType);
        const Location location = tokens[index].location;
        ++index;
        return IdentifierAstNode{value, location};
    }

    interpreter::type::PrimitiveType
    Parser::consumePrimitiveTypeName(const std::vector<token::Token> &tokens, size_t &index) {
        assertToken<token::Identifier>(tokens, index);
        auto [value] = std::get<token::Identifier>(tokens[index].tokenType);
        Location location = tokens[index].location;
        ++index;
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

    interpreter::type::Type Parser::parseType(const std::vector<token::Token> &tokens, size_t &index) {
        interpreter::type::PrimitiveType typ = consumePrimitiveTypeName(tokens, index);
        std::vector types{typ};
        while (std::holds_alternative<token::Arrow>(tokens[index].tokenType)) {
            assertAndConsume<token::Arrow>(tokens, index);
            types.push_back(consumePrimitiveTypeName(tokens, index));
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

    Expression Parser::parseExpression(const std::vector<token::Token> &tokens, size_t &index) {
        const auto &tok = tokens[index];
        const Location location = tok.location;
        return std::visit([&]<typename T0>(T0 &&) {
            using T = std::decay_t<T0>;
            if (std::is_same_v<T, token::Identifier>) {
                ++index;
                const auto value = std::get<token::Identifier>(tok.tokenType).value;
                return Expression(IdentifierAstNode{value, location});
            }
            if (std::is_same_v<T, token::String>) {
                ++index;
                const auto value = unescapeString(std::get<token::String>(tok.tokenType).value);
                return Expression(StringAstNode{value, location});
            }
            if (std::is_same_v<T, token::Float>) {
                ++index;
                const auto value = std::get<token::Float>(tok.tokenType).value;
                return Expression(FloatAstNode{value, location});
            }
            if (std::is_same_v<T, token::BackwardSlash>) {
                return Expression(parseLambdaExpression(tokens, index));
            }
            if (std::is_same_v<T, token::OpenParenthesis>) {
                return Expression(parseFunctionApplication(tokens, index));
            }
            optionsValue.logger.error(location, "syntax error: unexpected token ", tok.toString());
        }, tok.tokenType);
    }

    LambdaExpression Parser::parseLambdaExpression(const std::vector<token::Token> &tokens, size_t &index) {
        Location location = tokens[index].location;
        assertAndConsume<token::BackwardSlash>(tokens, index);
        IdentifierAstNode argument = consumeIdentifier(tokens, index);
        assertAndConsume<token::Colon>(tokens, index);
        interpreter::type::Type argumentType = parseType(tokens, index);
        assertAndConsume<token::Dot>(tokens, index);
        Expression expression = parseExpression(tokens, index);
        return LambdaExpression{
            argument, argumentType, std::make_unique<Expression>(std::move(expression)), location
        };
    }

    FunctionApplication Parser::parseFunctionApplication(const std::vector<token::Token> &tokens, size_t &index) {
        const Location location = tokens[index].location;
        assertAndConsume<token::OpenParenthesis>(tokens, index);
        const IdentifierAstNode functionName = consumeIdentifier(tokens, index);
        std::vector<std::unique_ptr<Expression> > arguments;
        while (!std::holds_alternative<token::CloseParenthesis>(tokens[index].tokenType)) {
            arguments.push_back(std::make_unique<Expression>(std::move(parseExpression(tokens, index))));
        }
        assertAndConsume<token::CloseParenthesis>(tokens, index);
        return FunctionApplication{functionName, std::move(arguments), location};
    }

    DefinitionAstNode Parser::parseDefinitionAstNode(const std::vector<token::Token> &tokens, size_t &index) {
        Location location = tokens[index].location;
        IdentifierAstNode definitionName = consumeIdentifier(tokens, index);
        assertAndConsume<token::Colon>(tokens, index);
        interpreter::type::Type definitionType = parseType(tokens, index);
        assertAndConsume<token::Equal>(tokens, index);
        Expression expression = parseExpression(tokens, index);
        return DefinitionAstNode{definitionName, definitionType, std::move(expression), location};
    }

    static std::string getAbsolutePath(const std::string &path) {
        return std::filesystem::absolute(path).string();
    }

    static void processUseFile(std::vector<AstNode> &nodes, const std::string &filepath) {
        const std::string absolutePath = getAbsolutePath(filepath);
        if (loadedFiles.contains(absolutePath)) {
            // Circular Dependency or Duplicate Load
            return;
        }
        loadedFiles.insert(absolutePath);
        auto lexerValue = Lexer::fromFile(filepath, optionsValue);
        const auto tokens = lexerValue.lex();
        if (optionsValue.debug) {
            for (const auto &token: tokens) {
                optionsValue.logger.debug(token);
            }
        }
        for (Parser parser(tokens, optionsValue); auto &astNode: parser.program.astNodes) {
            nodes.emplace_back(std::move(astNode));
        }
    }

    std::vector<AstNode> Parser::build(const std::vector<token::Token> &tokens, size_t &index) {
        std::vector<AstNode> nodes;
        while (!std::holds_alternative<token::Eof>(tokens[index].tokenType)) {
            const token::Token &token = tokens[index];
            std::visit([&]<typename T0>(T0 &&) {
                using T = std::decay_t<T0>;
                if constexpr (std::is_same_v<T, token::Identifier>) {
                    if (const auto identifierValue = std::get<token::Identifier>(tokens[index].tokenType).value;
                        identifierValue == "use") {
                        ++index; // consume "use"
                        assertToken<token::String>(tokens, index);
                        const std::string filepath = unescapeString(
                            std::get<token::String>(tokens[index].tokenType).value);
                        ++index; // consume <filepath>
                        processUseFile(nodes, filepath);
                    } else {
                        nodes.push_back(AstNode{std::move(parseDefinitionAstNode(tokens, index))});
                    }
                } else if constexpr (std::is_same_v<T, token::String> ||
                                     std::is_same_v<T, token::Float> ||
                                     std::is_same_v<T, token::BackwardSlash> ||
                                     std::is_same_v<T, token::OpenParenthesis>) {
                    nodes.push_back(AstNode{std::move(parseExpression(tokens, index))});
                } else {
                    optionsValue.logger.error(token.location, "syntax error: unexpected token ", token.toString());
                }
            }, token.tokenType);
        }
        return nodes;
    }
}

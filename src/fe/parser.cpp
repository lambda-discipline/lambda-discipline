#include <lbd/fe/loc.h>
#include <lbd/fe/lexer.h>
#include <lbd/fe/parser.h>
#include <lbd/utils/string_escape.h>
#include <unordered_set>
#include <filesystem>

namespace fe::parser {
    static std::unordered_set<std::string> loadedFiles;
    static options::Options optionsValue;

    Parser::Parser(const std::vector<token::Token> &tokens, const options::Options options_) {
        optionsValue = options_;
        size_t index = 0;
        program = ast::Program{build(tokens, index)};
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

    ast::IdentifierAstNode Parser::consumeIdentifier(const std::vector<token::Token> &tokens, size_t &index) {
        assertToken<token::Identifier>(tokens, index);
        auto [value] = std::get<token::Identifier>(tokens[index].tokenType);
        const loc::Loc location = tokens[index].location;
        ++index;
        return ast::IdentifierAstNode{value, location};
    }

    intp::types::PrimitiveType
    Parser::consumePrimitiveTypeName(const std::vector<token::Token> &tokens, size_t &index) {
        assertToken<token::Identifier>(tokens, index);
        auto [value] = std::get<token::Identifier>(tokens[index].tokenType);
        loc::Loc location = tokens[index].location;
        ++index;
        if (value == "Float") {
            return intp::types::PrimitiveType{intp::types::PrimitiveType::Type::Float};
        }
        if (value == "Str") {
            return intp::types::PrimitiveType{intp::types::PrimitiveType::Type::String};
        }
        if (value == "Any") {
            return intp::types::PrimitiveType{intp::types::PrimitiveType::Type::Any};
        }
        return intp::types::PrimitiveType{intp::types::PrimitiveType::Type::Custom, std::move(value)};
    }

    intp::types::Type Parser::parseType(const std::vector<token::Token> &tokens, size_t &index) {
        intp::types::PrimitiveType typ = consumePrimitiveTypeName(tokens, index);
        std::vector types{typ};
        while (std::holds_alternative<token::Arrow>(tokens[index].tokenType)) {
            assertAndConsume<token::Arrow>(tokens, index);
            types.push_back(consumePrimitiveTypeName(tokens, index));
        }
        if (types.size() == 1) {
            return typ;
        }
        // Construct Right Associative AST
        intp::types::Type currentType = types.back();
        for (int typeIndex = static_cast<int>(types.size()) - 2; typeIndex >= 0; --typeIndex) {
            auto nextCompoundType = std::make_shared<intp::types::CompoundType>();
            nextCompoundType->leftType = types[typeIndex];
            nextCompoundType->rightType = currentType;
            currentType = nextCompoundType;
        }
        return currentType;
    }

    ast::Expression Parser::parseExpression(const std::vector<token::Token> &tokens, size_t &index) {
        const auto &tok = tokens[index];
        const loc::Loc location = tok.location;
        return std::visit([&]<typename T0>(T0 &&) {
            using T = std::decay_t<T0>;
            if (std::is_same_v<T, token::Identifier>) {
                ++index;
                const auto value = std::get<token::Identifier>(tok.tokenType).value;
                return ast::Expression(ast::IdentifierAstNode{value, location});
            }
            if (std::is_same_v<T, token::String>) {
                ++index;
                const auto value = unescapeString(std::get<token::String>(tok.tokenType).value);
                return ast::Expression(ast::StringAstNode{value, location});
            }
            if (std::is_same_v<T, token::Float>) {
                ++index;
                const auto value = std::get<token::Float>(tok.tokenType).value;
                return ast::Expression(ast::FloatAstNode{value, location});
            }
            if (std::is_same_v<T, token::BackwardSlash>) {
                return ast::Expression(parseLambdaExpression(tokens, index));
            }
            if (std::is_same_v<T, token::OpenParenthesis>) {
                return ast::Expression(parseFunctionApplication(tokens, index));
            }
            optionsValue.logger.error(location, "syntax error: unexpected token ", tok.toString());
        }, tok.tokenType);
    }

    ast::LambdaExpression Parser::parseLambdaExpression(const std::vector<token::Token> &tokens, size_t &index) {
        loc::Loc location = tokens[index].location;
        assertAndConsume<token::BackwardSlash>(tokens, index);
        ast::IdentifierAstNode argument = consumeIdentifier(tokens, index);
        assertAndConsume<token::Colon>(tokens, index);
        intp::types::Type argumentType = parseType(tokens, index);
        assertAndConsume<token::Dot>(tokens, index);
        ast::Expression expression = parseExpression(tokens, index);
        return ast::LambdaExpression{
            argument, argumentType, std::make_unique<ast::Expression>(std::move(expression)), location
        };
    }

    ast::FunctionApplication Parser::parseFunctionApplication(const std::vector<token::Token> &tokens, size_t &index) {
        const loc::Loc location = tokens[index].location;
        assertAndConsume<token::OpenParenthesis>(tokens, index);
        const ast::IdentifierAstNode functionName = consumeIdentifier(tokens, index);
        std::vector<std::unique_ptr<ast::Expression> > arguments;
        while (!std::holds_alternative<token::CloseParenthesis>(tokens[index].tokenType)) {
            arguments.push_back(std::make_unique<ast::Expression>(std::move(parseExpression(tokens, index))));
        }
        assertAndConsume<token::CloseParenthesis>(tokens, index);
        return ast::FunctionApplication{functionName, std::move(arguments), location};
    }

    ast::DefinitionAstNode Parser::parseDefinitionAstNode(const std::vector<token::Token> &tokens, size_t &index) {
        loc::Loc location = tokens[index].location;
        ast::IdentifierAstNode definitionName = consumeIdentifier(tokens, index);
        assertAndConsume<token::Colon>(tokens, index);
        intp::types::Type definitionType = parseType(tokens, index);
        assertAndConsume<token::Equal>(tokens, index);
        ast::Expression expression = parseExpression(tokens, index);
        return ast::DefinitionAstNode{definitionName, definitionType, std::move(expression), location};
    }

    static std::string getAbsolutePath(const std::string &path) {
        return std::filesystem::absolute(path).string();
    }

    static void processUseFile(std::vector<ast::AstNode> &nodes, const std::string &filepath) {
        const std::string absolutePath = getAbsolutePath(filepath);
        if (loadedFiles.contains(absolutePath)) {
            // Circular Dependency or Duplicate Load
            return;
        }
        loadedFiles.insert(absolutePath);
        auto lexerValue = lexer::Lexer(filepath, lexer::FromFile{}, optionsValue);
        const auto tokens = lexerValue.lexAll();
        if (optionsValue.debug) {
            for (const auto &token: tokens) {
                optionsValue.logger.debug(token);
            }
        }
        for (Parser parser(tokens, optionsValue); auto &node: parser.program.nodes) {
            nodes.emplace_back(std::move(node));
        }
    }

    std::vector<ast::AstNode> Parser::build(const std::vector<token::Token> &tokens, size_t &index) {
        std::vector<ast::AstNode> nodes;
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
                        nodes.push_back(ast::AstNode{std::move(parseDefinitionAstNode(tokens, index))});
                    }
                } else if constexpr (std::is_same_v<T, token::String> ||
                                     std::is_same_v<T, token::Float> ||
                                     std::is_same_v<T, token::BackwardSlash> ||
                                     std::is_same_v<T, token::OpenParenthesis>) {
                    nodes.push_back(ast::AstNode{std::move(parseExpression(tokens, index))});
                } else {
                    optionsValue.logger.error(token.location, "syntax error: unexpected token ", token.toString());
                }
            }, token.tokenType);
        }
        return nodes;
    }
}

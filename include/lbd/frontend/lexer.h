#pragma once

#include <lbd/frontend/location.h>
#include <lbd/frontend/token.h>
#include <lbd/options.h>
#include <string>
#include <vector>

namespace frontend {
    struct Lexer {
        static Lexer fromFile(const std::string &filepath, context::Options options = {});

        static Lexer fromRepl(const std::string &source, context::Options options = {});

        // TODO: Add fromStdin

        token::Token nextToken();

        std::vector<token::Token> lex();

    private:
        explicit Lexer(std::string filepath, std::string source, context::Options options = {});

        std::string source;
        size_t position = 0;
        size_t row = 1;
        size_t col = 1;
        const std::string filepath;
        const context::Options options;

        [[nodiscard]] char peek() const;

        char get();

        [[nodiscard]] bool isEof() const;

        [[nodiscard]] Location getCurrentLocation() const;
    };
}

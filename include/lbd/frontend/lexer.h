#pragma once

#include <lbd/frontend/location.h>
#include <lbd/frontend/token.h>
#include <lbd/options.h>
#include <string>
#include <vector>

namespace frontend {
    struct FromFile {
    };

    struct FromRepl {
    };

    struct Lexer {
        // TODO: Make factory instead of these constructors.
        Lexer(const std::string &filepath, FromFile, global::Options options_ = {});

        Lexer(std::string str, FromRepl, global::Options options_ = {});

        token::Token nextToken();

        // TODO: Find a better name for this.
        std::vector<token::Token> lexAll();

    private:
        std::string source;
        size_t position = 0;
        size_t row = 1;
        size_t col = 1;
        std::string filepath;

        [[nodiscard]] char peek() const;

        char get();

        [[nodiscard]] bool isEof() const;

        [[nodiscard]] Location getCurrentLocation() const;
    };
}

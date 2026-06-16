#include <fstream>
#include <sstream>
#include <lbd/runtime/builtin-modules/BuiltinModuleIO.hpp>

namespace lbd::runtime::builtins
{
  // Prints Argument to stdout and returns 0.
  NativeFunction makePrint(Context &context)
  {
    const std::string name = "print";
    const auto signature = functionType(
      {simpleType(type::TypeTag::Any)},
      nullptr,
      true
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        for (auto &argument: arguments)
        {
          const Value &value = argument->force(context);
          std::cout << value;
        }
        return std::make_pair(Value{static_cast<double>(0)}, ResultOptions{.sideEffects = true});
      }
    };
  }

  NativeFunction makeSlurpFile(Context &context)
  {
    const std::string name = "slurpFile";
    const auto signature = functionType(
      {simpleType(type::TypeTag::String)},
      simpleType(type::TypeTag::String)
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context);
        const auto &path = std::get<std::string>(argument0);
        std::ifstream file(path, std::ios::in | std::ios::binary);
        if (!file)
        {
          context.getDiagnosticEmitter().error(
            arguments[0]->getRange(),
            diagnostics::DiagnosticId::IO_COULD_NOT_OPEN_FILE, path
          );
        }
        std::ostringstream buffer;
        buffer << file.rdbuf();
        return {Value{buffer.str()}, ResultOptions{}};
      }
    };
  }

  NativeFunction makeLines(Context &context)
  {
    const std::string name = "lines";
    const auto signature = functionType(
      {simpleType(type::TypeTag::String)},
      listType()
    );
    return {
      name, signature, [&context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                  const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context);
        const auto input = std::get<std::string>(argument0);
        // Normalize all line endings to '\n'
        std::string normalized;
        normalized.reserve(input.size());
        for (size_t i = 0; i < input.size(); ++i)
        {
          if (input[i] == '\r')
          {
            if (i + 1 < input.size() && input[i + 1] == '\n')
            {
              // CRLF -> LF
              normalized.push_back('\n');
              ++i;
            } else
            {
              // Lone CR -> LF
              normalized.push_back('\n');
            }
          } else
          {
            normalized.push_back(input[i]);
          }
        }
        std::istringstream inputStringStream(normalized);
        std::string line;
        std::vector<Value> result;
        while (std::getline(inputStringStream, line, '\n'))
        {
          result.emplace_back(line);
        }
        return {Value{std::make_shared<List>(std::move(result))}, ResultOptions{}};
      }
    };
  }

  NativeFunction makeSplit(Context &context)
  {
    const std::string name = "split";
    const auto signature = functionType(
      {simpleType(type::TypeTag::String), simpleType(type::TypeTag::String)},
      listType()
    );
    return {
      name, signature, [&name, &context](const std::vector<std::shared_ptr<Thunk>> &arguments,
                                         const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions>
      {
        const Value &argument0 = arguments[0]->force(context); // string
        const Value &argument1 = arguments[1]->force(context); // delimiter
        const auto &input = std::get<std::string>(argument0);
        const auto &delimiter = std::get<std::string>(argument1);
        if (delimiter.empty())
        {
          context.getDiagnosticEmitter().error(
            arguments[1]->getRange(),
            diagnostics::DiagnosticId::INVALID_INPUTS, name, "Delimiter cannot be empty."
          );
        }
        std::vector<Value> result;
        size_t start = 0;
        size_t position = 0;
        while ((position = input.find(delimiter, start)) != std::string::npos)
        {
          result.emplace_back(input.substr(start, position - start));
          start = position + delimiter.size();
        }
        result.emplace_back(input.substr(start));
        return {Value{std::make_shared<List>(List{std::move(result)})}, ResultOptions{}};
      }
    };
  }
}
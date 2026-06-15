#include <CLI/CLI.hpp>
#include <lbd/Context.hpp>

#include <lbd/cli/Registry.hpp>
#include <lbd/cli/commands/RunCommand.hpp>
#include <lbd/cli/commands/DocsCommand.hpp>
#include <lbd/cli/commands/REPLCommand.hpp>

int main(const int argc, char **argv)
{
  lbd::Context context(std::cerr);
  CLI::App app{"Functional Programming Language based on Lambda Calculus"};
  app.add_flag("-d,--debug", context.getOptions().debug);

  lbd::cli::CommandRegistry registry;
  registry.add(std::make_unique<lbd::cli::RunCommand>());
  registry.add(std::make_unique<lbd::cli::DocsCommand>());
  registry.add(std::make_unique<lbd::cli::REPLCommand>());
  registry.registerCommands(app);

  CLI11_PARSE(app, argc, argv);

  return registry.dispatch(context);
}

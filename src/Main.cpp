#include <iostream>
#include <string>
#include <lbd/Docs.hpp>
#include <lbd/Driver.hpp>
#include <lbd/REPL.hpp>
#include <lbd/cmd/Command.hpp>
#include <iomanip>

const std::string &programName = "lbd";

int main(const int argc, char **argv)
{
  const auto &[filepath, show_help, repl, debug, generateDocs] = lbd::cmd::parseArguments(argc, argv, programName);

  if (show_help)
  {
    lbd::cmd::printHelp(std::cout, argv[0]);
    return EXIT_SUCCESS;
  }

  if (generateDocs)
  {
    lbd::dumpDocs(std::cout);
    return EXIT_SUCCESS;
  }

  lbd::Context context(std::cerr);

  if (repl)
  {
    lbd::repl::loop(context, debug);
    return EXIT_SUCCESS;
  }

  const lbd::source::BufferId bufferId = context.getBufferManager().loadFile(*filepath);
  auto result = lbd::Driver(context).run(bufferId);
  return EXIT_SUCCESS;
}

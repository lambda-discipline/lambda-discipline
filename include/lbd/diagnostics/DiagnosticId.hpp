#pragma once

namespace lbd::diagnostics
{
  enum class DiagnosticId
  { 

#define DIAGNOSTIC(id, message) id,
#include <lbd/resources/Diagnostics.def>
#undef DIAGNOSTIC
  COUNT
};

}
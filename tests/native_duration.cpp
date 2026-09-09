#include "../native/DurationPolicy.h"
#include <cassert>
#include <limits>
int main() {
  // Real catalog / MPModelSong / decoded asset observations for the same store ID.
  assert(ONMPlaybackDurationMatches(313.577, 313577, false));
  assert(ONMPlaybackDurationMatches(313.0, 313577, true));
  assert(ONMPlaybackDurationMatches(313.624, 313577, true));
  // Store metadata must retain exact catalog duration; playback rules cannot leak here.
  assert(!ONMPlaybackDurationMatches(313.0, 313577, false));
  assert(!ONMPlaybackDurationMatches(313.624, 313577, false));
  // Quantization is floor only, and fractional asset offsets are tightly bounded.
  assert(!ONMPlaybackDurationMatches(314.0, 313577, true));
  assert(!ONMPlaybackDurationMatches(312.0, 313577, true));
  assert(!ONMPlaybackDurationMatches(313.4, 313577, true));
  assert(!ONMPlaybackDurationMatches(313.8, 313577, true));
  assert(!ONMPlaybackDurationMatches(0, 313577, true));
  assert(!ONMPlaybackDurationMatches(313.577, 0, true));
  assert(!ONMPlaybackDurationMatches(std::numeric_limits<double>::quiet_NaN(), 313577, true));
  assert(!ONMPlaybackDurationMatches(std::numeric_limits<double>::infinity(), 313577, true));
}

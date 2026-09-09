bool ONMPlaybackDurationMatches(double seconds, long expected, bool playbackModel) {
  if (!(seconds > 0 && seconds < 86400) || expected <= 0 || expected >= 86400000)
    return false;
  long millis = (long)(seconds * 1000 + 0.5);
  if (millis == expected)
    return true;
  if (!playbackModel)
    return false;
  if (seconds == (double)(long)seconds)
    return (long)seconds == expected / 1000;
  long difference = millis - expected;
  return difference >= -100 && difference <= 100;
}

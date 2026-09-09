extern "C" void *dlsym(void *, const char *);
extern "C" char *strdup(const char *);
void *ONBMsg;
void *ONBGet(void *o, const char *s) {
  return ((void *(*)(void *, SEL))ONBMsg)(o, sel_registerName(s));
}
void *ONBStr(const char *s) {
  return ((void *(*)(void *, SEL, const char *))ONBMsg)(
      (void *)objc_getClass("NSString"), sel_registerName("stringWithUTF8String:"), s);
}
bool ONBKind(void *o, const char *c) {
  return o && ((bool (*)(void *, SEL, Class))ONBMsg)(o, sel_registerName("isKindOfClass:"),
                                                     (Class)objc_getClass(c));
}
void *ONBKey(void *o, const char *k) {
  return ((void *(*)(void *, SEL, void *))ONBMsg)(o, sel_registerName("objectForKey:"), ONBStr(k));
}
bool ONBEqual(void *a, void *b) {
  return a && b && ((bool (*)(void *, SEL, void *))ONBMsg)(a, sel_registerName("isEqual:"), b);
}
extern "C" long time(long *);
Method ONMMethods[8];
IMP ONMOriginals[8];
IMP ONMHooks[8];
bool ONMInstalled;
char *ONMPath;
void *ONMMap;
long ONMNextRead;
long ONMFileSeconds;
long ONMFileNanos;
double ONMExpires;
unsigned long ONMCalls;
unsigned long ONMChanges;
void *ONMNames(const char *onm_type, void *onm_ident) {
  if (!onm_ident)
    return 0;
  id onm_lock = (id)objc_getClass("ONMusicModelNamesV2");
  objc_sync_enter(onm_lock);
  long onm_now = time(0);
  struct stat onm_stat;
  bool onm_file_ok = stat(ONMPath, &onm_stat) == 0;
  bool onm_file_changed = onm_file_ok && (onm_stat.st_mtimespec.tv_sec != ONMFileSeconds ||
                                          onm_stat.st_mtimespec.tv_nsec != ONMFileNanos);
  if (onm_file_changed || onm_now >= ONMNextRead) {
    ONMNextRead = onm_now + 2;
    void *onm_data = ((void *(*)(void *, SEL, void *))ONBMsg)(
        (void *)objc_getClass("NSData"), sel_registerName("dataWithContentsOfFile:"),
        ONBStr(ONMPath));
    if (onm_data &&
        ((unsigned long (*)(void *, SEL))ONBMsg)(onm_data, sel_registerName("length")) <= 8388608) {
      void *onm_document = ((void *(*)(void *, SEL, void *, unsigned long, void **))ONBMsg)(
          (void *)objc_getClass("NSJSONSerialization"),
          sel_registerName("JSONObjectWithData:options:error:"), onm_data, 0, 0);
      if (ONBKind(onm_document, "NSDictionary") &&
          ONBKind(ONBKey(onm_document, "names"), "NSDictionary")) {
        void *onm_new = ONBGet(ONBKey(onm_document, "names"), "retain");
        ONBGet(ONMMap, "release");
        ONMMap = onm_new;
        if (onm_file_ok) {
          ONMFileSeconds = onm_stat.st_mtimespec.tv_sec;
          ONMFileNanos = onm_stat.st_mtimespec.tv_nsec;
        }
        ONMExpires = ((double (*)(void *, SEL))ONBMsg)(ONBKey(onm_document, "expires"),
                                                       sel_registerName("doubleValue"));
      }
    }
  }
  void *onm_result = 0;
  if (ONMExpires > onm_now) {
    void *onm_key = ((void *(*)(void *, SEL, void *))ONBMsg)(
        ONBStr(onm_type), sel_registerName("stringByAppendingString:"),
        ONBGet(onm_ident, "description"));
    onm_result = ONBGet(ONBKey(ONMMap, (const char *)ONBGet(onm_key, "UTF8String")), "retain");
  }
  objc_sync_exit(onm_lock);
  return ONBGet(onm_result, "autorelease");
}
bool ONMDurationMatches(void *onm_self, void *onm_names, int onm_index) {
  if (onm_index == 3) {
    void *onm_key = ONBGet((void *)object_getClass((id)onm_self), "__duration_KEY");
    if (!onm_key || !((bool (*)(void *, SEL, void *))ONBMsg)(
                        onm_self, sel_registerName("hasLoadedValueForKey:"), onm_key))
      return false;
  }
  double onm_seconds = ((double (*)(void *, SEL))ONBMsg)(onm_self, sel_registerName("duration"));
  long onm_expected = ((long (*)(void *, SEL))ONBMsg)(ONBKey(onm_names, "durationInMillis"),
                                                      sel_registerName("longLongValue"));
  return ONMPlaybackDurationMatches(onm_seconds, onm_expected, onm_index == 3);
}
void *ONMReadName(void *onm_self, SEL onm_selector, int onm_index) {
  void *onm_original = ((void *(*)(void *, SEL))ONMOriginals[onm_index])(onm_self, onm_selector);
  if (!ONMInstalled || !ONBKind(onm_original, "NSString"))
    return onm_original;
  __atomic_fetch_add(&ONMCalls, 1, __ATOMIC_RELAXED);
  void *onm_ident = 0, *onm_names = 0;
  if ((onm_index < 3 || onm_index >= 6)) {
    onm_ident = ONBGet(onm_self, "storeID");
    onm_names = ONMNames("songs:", onm_ident);
    if (onm_names && !ONMDurationMatches(onm_self, onm_names, onm_index))
      return onm_original;
    if (!onm_names)
      onm_names = ONMNames("albums:", onm_ident);
    if (!onm_names)
      onm_names = ONMNames("artists:", onm_ident);
  } else {
    void *onm_identifiers = ONBGet(onm_self, "identifiers");
    long onm_id = ((long (*)(void *, SEL))ONBMsg)(onm_identifiers, sel_registerName("storeAdamID"));
    if (onm_id <= 0)
      onm_id = ((long (*)(void *, SEL))ONBMsg)(onm_identifiers,
                                               sel_registerName("storeSubscriptionAdamID"));
    if (onm_id <= 0)
      onm_id = ((long (*)(void *, SEL))ONBMsg)(onm_identifiers, sel_registerName("adamID"));
    if (onm_id <= 0)
      return onm_original;
    onm_ident = ((void *(*)(void *, SEL, long))ONBMsg)(
        (void *)objc_getClass("NSNumber"), sel_registerName("numberWithLongLong:"), onm_id);
    onm_names = ONMNames(onm_index == 3   ? "songs:"
                         : onm_index == 4 ? "albums:"
                                          : "artists:",
                         onm_ident);
    if (onm_index == 3 && onm_names && !ONMDurationMatches(onm_self, onm_names, onm_index))
      return onm_original;
  }
  const char *onm_field = onm_index == 1 ? "artistName" : onm_index == 2 ? "albumName" : "name";
  void *onm_value = onm_names ? ONBKey(onm_names, onm_field) : 0;
  if (ONBKind(onm_value, "NSString") && !ONBEqual(onm_value, onm_original)) {
    __atomic_fetch_add(&ONMChanges, 1, __ATOMIC_RELAXED);
    return onm_value;
  }
  return onm_original;
}
void *ONM0(void *s, SEL c) {
  return ONMReadName(s, c, 0);
}
void *ONM1(void *s, SEL c) {
  return ONMReadName(s, c, 1);
}
void *ONM2(void *s, SEL c) {
  return ONMReadName(s, c, 2);
}
void *ONM3(void *s, SEL c) {
  return ONMReadName(s, c, 3);
}
void *ONM4(void *s, SEL c) {
  return ONMReadName(s, c, 4);
}
void *ONM5(void *s, SEL c) {
  return ONMReadName(s, c, 5);
}
void *ONM6(void *s, SEL c) {
  return ONMReadName(s, c, 6);
}
void *ONM7(void *s, SEL c) {
  return ONMReadName(s, c, 7);
}
int ONMRemove(id s, SEL c) {
  if (!ONMInstalled)
    return 0;
  ONMInstalled = false;
  for (int i = 0; i < 8; i++)
    if (method_getImplementation(ONMMethods[i]) == ONMHooks[i])
      method_setImplementation(ONMMethods[i], ONMOriginals[i]);
  return 1;
}
int ONMEnable(id s, SEL c) {
  if (ONMInstalled)
    return 1;
  for (int i = 0; i < 8; i++)
    if (method_getImplementation(ONMMethods[i]) != ONMOriginals[i])
      return -7;
  for (int i = 0; i < 8; i++)
    method_setImplementation(ONMMethods[i], ONMHooks[i]);
  ONMInstalled = true;
  return 1;
}
unsigned long ONMActive(id s, SEL c) {
  return ONMInstalled ? 1 : 0;
}
unsigned long ONMVersion(id s, SEL c) {
  return 2026090901;
}
unsigned long ONMCount(id s, SEL c) {
  return __atomic_load_n(&ONMChanges, __ATOMIC_RELAXED);
}
unsigned long ONMCallCount(id s, SEL c) {
  return __atomic_load_n(&ONMCalls, __ATOMIC_RELAXED);
}
int ONMInstall(const char *path) {
  if (objc_getClass("ONMusicModelNamesV2"))
    return -2;
  const char *classes[] = {"MPStoreItemMetadata", "MPStoreItemMetadata", "MPStoreItemMetadata",
                           "MPModelSong",         "MPModelAlbum",        "MPModelArtist",
                           "MPStoreItemMetadata", "MPStoreItemMetadata"};
  const char *selectors[] = {"name",  "artistName", "collectionName", "title",
                             "title", "name",       "nameRaw",        "shortName"};
  IMP hooks[] = {(IMP)ONM0, (IMP)ONM1, (IMP)ONM2, (IMP)ONM3,
                 (IMP)ONM4, (IMP)ONM5, (IMP)ONM6, (IMP)ONM7};
  for (int i = 0; i < 8; i++) {
    ONMMethods[i] =
        class_getInstanceMethod((Class)objc_getClass(classes[i]), sel_registerName(selectors[i]));
    if (!ONMMethods[i])
      return -3;
  }
  ONMPath = strdup(path);
  ONBMsg = __builtin_ptrauth_sign_unauthenticated(
      __builtin_ptrauth_strip(dlsym((void *)-2, "objc_msgSend"), 0), 0, 0);
  Class control =
      objc_allocateClassPair((Class)objc_getClass("NSObject"), "ONMusicModelNamesV2", 0);
  if (!control)
    return -4;
  class_addMethod(object_getClass(control), sel_registerName("removeProbe"), (IMP)ONMRemove, "i@:");
  class_addMethod(object_getClass(control), sel_registerName("enableProbe"), (IMP)ONMEnable, "i@:");
  class_addMethod(object_getClass(control), sel_registerName("isInstalled"), (IMP)ONMActive, "Q@:");
  class_addMethod(object_getClass(control), sel_registerName("hookVersion"), (IMP)ONMVersion,
                  "Q@:");
  class_addMethod(object_getClass(control), sel_registerName("targetCount"), (IMP)ONMCount, "Q@:");
  class_addMethod(object_getClass(control), sel_registerName("parseCount"), (IMP)ONMCallCount,
                  "Q@:");
  objc_registerClassPair(control);
  for (int i = 0; i < 8; i++) {
    ONMHooks[i] = hooks[i];
    ONMOriginals[i] = method_setImplementation(ONMMethods[i], hooks[i]);
  }
  ONMInstalled = true;
  return 1;
}

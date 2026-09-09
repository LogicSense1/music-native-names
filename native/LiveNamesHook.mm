extern "C" void *dlsym(void *, const char *);
extern "C" int close(int);
void *ONVMsg;
unsigned long ONVCalls;
unsigned long ONVChanges;
Method ONVMethods[3];
IMP ONVOrig[3];
IMP ONVHooks[3];
bool ONVInstalled;
void *ONVGet(void *onv_o, const char *onv_s) {
  return ((void *(*)(void *, SEL))ONVMsg)(onv_o, sel_registerName(onv_s));
}
bool ONVKind(void *onv_o, const char *onv_c) {
  return onv_o && ((bool (*)(void *, SEL, Class))ONVMsg)(onv_o, sel_registerName("isKindOfClass:"),
                                                         (Class)objc_getClass(onv_c));
}
bool ONVEligible(void *onv_url) {
  const char *onv_h = (const char *)ONVGet(ONVGet(onv_url, "host"), "UTF8String");
  const char *onv_p = (const char *)ONVGet(ONVGet(onv_url, "path"), "UTF8String");
  const char *onv_s = (const char *)ONVGet(ONVGet(onv_url, "scheme"), "UTF8String");
  if (!onv_h || !onv_p || !onv_s || strcmp(onv_s, "https") ||
      (strcmp(onv_h, "amp-api.music.apple.com") && strcmp(onv_h, "amp-api-edge.music.apple.com")))
    return false;
  bool onv_match =
      !strncmp(onv_p, "/v1/catalog/us/", 15) || !strcmp(onv_p, "/v1/editorial/us/groupings") ||
      !strcmp(onv_p, "/v1/recommendations/us") || !strcmp(onv_p, "/v1/me/recommendations") ||
      !strncmp(onv_p, "/v1/me/recommendations/", 23) || !strcmp(onv_p, "/v1/me/recent/played") ||
      !strcmp(onv_p, "/v1/me/history/heavy-rotation");
  return onv_match;
}
extern "C" int socket(int, int, int);
extern "C" int connect(int, const void *, unsigned int);
extern "C" int setsockopt(int, int, int, const void *, unsigned int);
extern "C" long read(int, void *, unsigned long);
extern "C" long write(int, const void *, unsigned long);
extern "C" void *malloc(unsigned long);
extern "C" void free(void *);
struct ONVAddress {
  unsigned char len;
  unsigned char family;
  unsigned short port;
  unsigned int addr;
  char pad[8];
};
struct ONVTimeval {
  long sec;
  int usec;
};
bool ONVTransfer(int onv_fd, void *onv_buf, unsigned long onv_length, bool onv_send) {
  char *onv_ptr = (char *)onv_buf;
  while (onv_length) {
    long onv_count =
        onv_send ? write(onv_fd, onv_ptr, onv_length) : read(onv_fd, onv_ptr, onv_length);
    if (onv_count <= 0)
      return false;
    onv_ptr += onv_count;
    onv_length -= onv_count;
  }
  return true;
}
void *ONVTransform(void *onv_data) {
  if (!ONVInstalled)
    return onv_data;
  __atomic_fetch_add(&ONVCalls, 1, __ATOMIC_RELAXED);
  if (!ONVKind(onv_data, "NSData"))
    return onv_data;
  unsigned long onv_length =
      ((unsigned long (*)(void *, SEL))ONVMsg)(onv_data, sel_registerName("length"));
  if (!onv_length || onv_length > 8388608)
    return onv_data;
  int onv_fd = socket(2, 1, 0);
  if (onv_fd < 0)
    return onv_data;
  int onv_one = 1;
  setsockopt(onv_fd, 65535, 0x1022, &onv_one, sizeof(onv_one));
  ONVTimeval onv_timeout;
  onv_timeout.sec = 7;
  onv_timeout.usec = 0;
  setsockopt(onv_fd, 65535, 0x1005, &onv_timeout, sizeof(onv_timeout));
  setsockopt(onv_fd, 65535, 0x1006, &onv_timeout, sizeof(onv_timeout));
  ONVAddress onv_address = {16, 2, 0xaa46, 0x0100007f, {0}};
  if (connect(onv_fd, &onv_address, sizeof(onv_address))) {
    close(onv_fd);
    return onv_data;
  }
  void *onv_bytes = ONVGet(onv_data, "bytes");
  unsigned long onv_resultLength = 0;
  bool onv_ok = ONVTransfer(onv_fd, &onv_length, 8, true) &&
                ONVTransfer(onv_fd, onv_bytes, onv_length, true) &&
                ONVTransfer(onv_fd, &onv_resultLength, 8, false);
  if (!onv_ok || !onv_resultLength || onv_resultLength > 8388608) {
    close(onv_fd);
    return onv_data;
  }
  void *onv_result = malloc(onv_resultLength);
  if (!onv_result) {
    close(onv_fd);
    return onv_data;
  }
  onv_ok = ONVTransfer(onv_fd, onv_result, onv_resultLength, false);
  close(onv_fd);
  void *onv_out = onv_data;
  if (onv_ok)
    onv_out = ((void *(*)(void *, SEL, void *, unsigned long))ONVMsg)(
        (void *)objc_getClass("NSMutableData"), sel_registerName("dataWithBytes:length:"),
        onv_result, onv_resultLength);
  free(onv_result);
  if (onv_out != onv_data)
    __atomic_fetch_add(&ONVChanges, 1, __ATOMIC_RELAXED);
  return onv_out;
}
extern "C" void _Block_object_assign(void *, const void *, const int);
extern "C" void _Block_object_dispose(const void *, const int);
struct ONVBlockDescriptor {
  unsigned long reserved;
  unsigned long size;
  void *copy;
  void *dispose;
  const char *signature;
};
struct ONVBlock {
  void *isa;
  int flags;
  int reserved;
  void *invoke;
  ONVBlockDescriptor *descriptor;
  void *completion;
};
ONVBlockDescriptor ONVDescriptor;
void *ONVStackClass;
void ONVBlockCopy(ONVBlock *onv_dst, ONVBlock *onv_src) {
  _Block_object_assign(&onv_dst->completion, onv_src->completion, 7);
}
void ONVBlockDispose(ONVBlock *onv_src) {
  _Block_object_dispose(onv_src->completion, 7);
}
void ONVBlockInvoke(ONVBlock *onv_block, void *onv_data, void *onv_response, void *onv_error) {
  if (!onv_error && ONVKind(onv_response, "NSHTTPURLResponse") &&
      ((long (*)(void *, SEL))ONVMsg)(onv_response, sel_registerName("statusCode")) == 200)
    onv_data = ONVTransform(onv_data);
  ONVBlock *onv_original = (ONVBlock *)onv_block->completion;
  void *onv_invoke = __builtin_ptrauth_auth_and_resign(onv_original->invoke, 0,
                                                       (unsigned long)&onv_original->invoke, 0, 0);
  ((void (*)(void *, void *, void *, void *))onv_invoke)(onv_original, onv_data, onv_response,
                                                         onv_error);
}
char ONVDataCacheKey;
void *ONVResultData(void *onv_self, SEL onv_cmd) {
  void *onv_original = ((void *(*)(void *, SEL))ONVOrig[0])(onv_self, onv_cmd);
  if (!ONVInstalled || !ONVKind(onv_original, "NSData"))
    return onv_original;
  void *onv_cached = (void *)objc_getAssociatedObject((id)onv_self, &ONVDataCacheKey);
  if (onv_cached)
    return onv_cached;
  void *onv_response = ONVGet(onv_self, "response");
  if (!ONVKind(onv_response, "NSHTTPURLResponse") ||
      ((long (*)(void *, SEL))ONVMsg)(onv_response, sel_registerName("statusCode")) != 200 ||
      !ONVEligible(ONVGet(onv_response, "URL")))
    return onv_original;
  void *onv_changed = ONVTransform(onv_original);
  objc_setAssociatedObject((id)onv_self, &ONVDataCacheKey, (id)onv_changed,
                           OBJC_ASSOCIATION_RETAIN);
  return onv_changed;
}
void *ONVFactory(void *onv_self, SEL onv_cmd, void *onv_input, void *onv_completion,
                 int onv_index) {
  void *onv_url = onv_index == 1 ? ONVGet(onv_input, "URL") : onv_input;
  if (!onv_completion || !ONVEligible(onv_url))
    return ((void *(*)(void *, SEL, void *, void *))ONVOrig[onv_index])(onv_self, onv_cmd,
                                                                        onv_input, onv_completion);
  ONVBlock onv_wrapped;
  onv_wrapped.isa = __builtin_ptrauth_sign_unauthenticated(
      ONVStackClass, 2, __builtin_ptrauth_blend_discriminator(&onv_wrapped.isa, 27361));
  onv_wrapped.flags = 0x42000000;
  onv_wrapped.reserved = 0;
  onv_wrapped.invoke = __builtin_ptrauth_sign_unauthenticated(
      __builtin_ptrauth_strip((void *)ONVBlockInvoke, 0), 0, (unsigned long)&onv_wrapped.invoke);
  onv_wrapped.descriptor = &ONVDescriptor;
  onv_wrapped.completion = onv_completion;
  return ((void *(*)(void *, SEL, void *, void *))ONVOrig[onv_index])(onv_self, onv_cmd, onv_input,
                                                                      &onv_wrapped);
}
void *ONVRequest(void *onv_s, SEL onv_c, void *onv_input, void *onv_completion) {
  return ONVFactory(onv_s, onv_c, onv_input, onv_completion, 1);
}
void *ONVURL(void *onv_s, SEL onv_c, void *onv_input, void *onv_completion) {
  return ONVFactory(onv_s, onv_c, onv_input, onv_completion, 2);
}
int ONVRemove(id onv_s, SEL onv_c) {
  if (!ONVInstalled)
    return 0;
  for (int onv_i = 0; onv_i < 3; onv_i++)
    if (method_getImplementation(ONVMethods[onv_i]) == ONVHooks[onv_i])
      method_setImplementation(ONVMethods[onv_i], ONVOrig[onv_i]);
  ONVInstalled = false;
  return 1;
}
unsigned long ONVVersion(id onv_s, SEL onv_c) {
  return 2026090812;
}
unsigned long ONVActive(id onv_s, SEL onv_c) {
  return ONVInstalled ? 1 : 0;
}
int ONVEnable(id onv_s, SEL onv_c) {
  if (ONVInstalled)
    return 1;
  for (int onv_i = 0; onv_i < 3; onv_i++)
    if (method_getImplementation(ONVMethods[onv_i]) != ONVOrig[onv_i])
      return -7;
  for (int onv_i = 0; onv_i < 3; onv_i++)
    method_setImplementation(ONVMethods[onv_i], ONVHooks[onv_i]);
  ONVInstalled = true;
  return 1;
}
unsigned long ONVCount(id onv_s, SEL onv_c) {
  return __atomic_load_n(&ONVChanges, __ATOMIC_RELAXED);
}
unsigned long ONVCallCount(id onv_s, SEL onv_c) {
  return __atomic_load_n(&ONVCalls, __ATOMIC_RELAXED);
}
int ONVInstall() {
  if (objc_getClass("ONMusicLiveNames"))
    return -2;
  ONVStackClass = dlsym((void *)-2, "_NSConcreteStackBlock");
  ONVDescriptor.reserved = 0;
  ONVDescriptor.size = sizeof(ONVBlock);
  ONVDescriptor.signature = "v32@?0@8@16@24";
  ONVDescriptor.copy = __builtin_ptrauth_sign_unauthenticated(
      __builtin_ptrauth_strip((void *)ONVBlockCopy, 0), 0, (unsigned long)&ONVDescriptor.copy);
  ONVDescriptor.dispose =
      __builtin_ptrauth_sign_unauthenticated(__builtin_ptrauth_strip((void *)ONVBlockDispose, 0), 0,
                                             (unsigned long)&ONVDescriptor.dispose);
  ONVMsg = __builtin_ptrauth_sign_unauthenticated(
      __builtin_ptrauth_strip(dlsym((void *)-2, "objc_msgSend"), 0), 0, 0);
  const char *onv_selectors[] = {
      "data", "dataTaskWithRequest:completionHandler:", "dataTaskWithURL:completionHandler:"};
  for (int onv_i = 0; onv_i < 3; onv_i++) {
    ONVMethods[onv_i] =
        class_getInstanceMethod((Class)objc_getClass(onv_i == 0 ? "AMSURLResult" : "NSURLSession"),
                                sel_registerName(onv_selectors[onv_i]));
    if (!ONVMethods[onv_i])
      return -3;
  }
  Class onv_control =
      objc_allocateClassPair((Class)objc_getClass("NSObject"), "ONMusicLiveNames", 0);
  if (!onv_control)
    return -4;
  class_addMethod(object_getClass(onv_control), sel_registerName("removeProbe"), (IMP)ONVRemove,
                  "i@:");
  class_addMethod(object_getClass(onv_control), sel_registerName("targetCount"), (IMP)ONVCount,
                  "Q@:");
  class_addMethod(object_getClass(onv_control), sel_registerName("parseCount"), (IMP)ONVCallCount,
                  "Q@:");
  class_addMethod(object_getClass(onv_control), sel_registerName("hookVersion"), (IMP)ONVVersion,
                  "Q@:");
  class_addMethod(object_getClass(onv_control), sel_registerName("isInstalled"), (IMP)ONVActive,
                  "Q@:");
  class_addMethod(object_getClass(onv_control), sel_registerName("enableProbe"), (IMP)ONVEnable,
                  "i@:");
  objc_registerClassPair(onv_control);
  ONVHooks[0] = (IMP)ONVResultData;
  ONVHooks[1] = (IMP)ONVRequest;
  ONVHooks[2] = (IMP)ONVURL;
  for (int onv_i = 0; onv_i < 3; onv_i++)
    ONVOrig[onv_i] = method_setImplementation(ONVMethods[onv_i], ONVHooks[onv_i]);
  ONVInstalled = true;
  return 1;
}

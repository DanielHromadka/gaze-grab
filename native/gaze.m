#import <AppKit/AppKit.h>
#import <AVFoundation/AVFoundation.h>
#import <Vision/Vision.h>
#import <ImageIO/ImageIO.h>
#import <ApplicationServices/ApplicationServices.h>
extern void *gaze_create(void);
extern char *gaze_process(void *,const char *);
extern void gaze_destroy(void *);
extern void gaze_free(char *);
static NSDictionary *Evaluate(void *tracker,NSDictionary *frame) {
    NSData *data=[NSJSONSerialization dataWithJSONObject:frame options:0 error:nil];
    NSString *json=[[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
    char *result=gaze_process(tracker,json.UTF8String); if (!result) return @{};
    NSData *output=[[NSString stringWithUTF8String:result] dataUsingEncoding:NSUTF8StringEncoding]; gaze_free(result);
    return [NSJSONSerialization JSONObjectWithData:output options:0 error:nil] ?: @{};
}
static NSDictionary *PointJSON(CGPoint p,float confidence) { return @{@"x":@(p.x),@"y":@(p.y),@"confidence":@(confidence)}; }
static NSArray *FacePoints(VNFaceLandmarkRegion2D *region,CGRect box,float confidence) {
    NSMutableArray *result=[NSMutableArray new];
    for (NSUInteger i=0;i<region.pointCount;i++) {
        CGPoint p=region.normalizedPoints[i];
        [result addObject:PointJSON(CGPointMake(box.origin.x+p.x*box.size.width,box.origin.y+p.y*box.size.height),confidence)];
    }
    return result;
}
static NSArray *JointNames(void) {
    return @[VNHumanHandPoseObservationJointNameWrist,VNHumanHandPoseObservationJointNameThumbCMC,VNHumanHandPoseObservationJointNameThumbMP,VNHumanHandPoseObservationJointNameThumbIP,VNHumanHandPoseObservationJointNameThumbTip,
        VNHumanHandPoseObservationJointNameIndexMCP,VNHumanHandPoseObservationJointNameIndexPIP,VNHumanHandPoseObservationJointNameIndexDIP,VNHumanHandPoseObservationJointNameIndexTip,
        VNHumanHandPoseObservationJointNameMiddleMCP,VNHumanHandPoseObservationJointNameMiddlePIP,VNHumanHandPoseObservationJointNameMiddleDIP,VNHumanHandPoseObservationJointNameMiddleTip,
        VNHumanHandPoseObservationJointNameRingMCP,VNHumanHandPoseObservationJointNameRingPIP,VNHumanHandPoseObservationJointNameRingDIP,VNHumanHandPoseObservationJointNameRingTip,
        VNHumanHandPoseObservationJointNameLittleMCP,VNHumanHandPoseObservationJointNameLittlePIP,VNHumanHandPoseObservationJointNameLittleDIP,VNHumanHandPoseObservationJointNameLittleTip];
}
@interface GGOverlay : NSView
@property(nonatomic,strong) NSDictionary *landmarksFrame,*evaluation;
@end
@implementation GGOverlay
- (BOOL)isOpaque {return NO;}
- (NSPoint)point:(NSDictionary *)point in:(NSRect)rect {
    return NSMakePoint(rect.origin.x+(1-[point[@"x"] doubleValue])*rect.size.width,rect.origin.y+[point[@"y"] doubleValue]*rect.size.height);
}
- (NSPoint)pair:(NSArray *)point in:(NSRect)rect {return [self point:@{@"x":point[0],@"y":point[1]} in:rect];}
- (void)drawRect:(NSRect)dirty {
    (void)dirty; NSDictionary *frame=self.landmarksFrame; if (!frame) return;
    double width=[frame[@"width"] doubleValue],height=[frame[@"height"] doubleValue];if (width<=0 || height<=0)return;
    double scale=MIN(self.bounds.size.width/width,self.bounds.size.height/height);
    NSRect rect=NSMakeRect((self.bounds.size.width-width*scale)/2,(self.bounds.size.height-height*scale)/2,width*scale,height*scale);
    [NSBezierPath clipRect:rect];
    for (NSDictionary *observed in frame[@"hands"]) {
    NSArray *hand=observed[@"points"];
    BOOL left=[observed[@"side"] isEqual:@"left"];
    NSColor *handColor=left ? NSColor.systemPurpleColor : NSColor.systemTealColor;
    if ([hand isKindOfClass:NSArray.class] && hand.count==21) {
        int edges[][2]={{0,1},{1,2},{2,3},{3,4},{0,5},{5,6},{6,7},{7,8},{5,9},{9,10},{10,11},{11,12},{9,13},{13,14},{14,15},{15,16},{13,17},{17,18},{18,19},{19,20},{17,0}};
        [handColor setStroke];
        NSBezierPath *bones=[NSBezierPath bezierPath];bones.lineWidth=2;
        for (NSUInteger i=0;i<sizeof(edges)/sizeof(edges[0]);i++) {
            NSDictionary *a=hand[edges[i][0]],*b=hand[edges[i][1]];
            if ([a[@"confidence"] doubleValue]<0.5 || [b[@"confidence"] doubleValue]<0.5)continue;
            [bones moveToPoint:[self point:a in:rect]];[bones lineToPoint:[self point:b in:rect]];
        }
        [bones stroke];
        for (NSUInteger i=0;i<hand.count;i++) if ([hand[i][@"confidence"] doubleValue]>=0.5) {
            BOOL tip=i==4 || i==8 || i==12;
            NSColor *color=tip ? ([self.evaluation[@"holding"] boolValue] ? NSColor.systemGreenColor : NSColor.systemOrangeColor) : handColor;
            [color setFill];NSPoint p=[self point:hand[i] in:rect];double r=tip ? 4 : 2.5;
            [[NSBezierPath bezierPathWithOvalInRect:NSMakeRect(p.x-r,p.y-r,r*2,r*2)] fill];
        }
    }
    if (hand.count==21) { NSPoint label=[self point:hand[0] in:rect];
        [(left ? @"LEVÁ" : [observed[@"side"] isEqual:@"right"] ? @"PRAVÁ" : @"?") drawAtPoint:label withAttributes:@{NSFontAttributeName:[NSFont boldSystemFontOfSize:12],NSForegroundColorAttributeName:handColor}]; }
    }
    NSArray *cursor=self.evaluation[@"control"][@"pointer"];
    if([cursor isKindOfClass:NSArray.class] && cursor.count==2) {
        NSPoint p=NSMakePoint(rect.origin.x+[cursor[0] doubleValue]*rect.size.width,rect.origin.y+(1-[cursor[1] doubleValue])*rect.size.height);
        NSBezierPath *cross=[NSBezierPath bezierPath];cross.lineWidth=2;
        [cross moveToPoint:NSMakePoint(p.x-9,p.y)];[cross lineToPoint:NSMakePoint(p.x+9,p.y)];
        [cross moveToPoint:NSMakePoint(p.x,p.y-9)];[cross lineToPoint:NSMakePoint(p.x,p.y+9)];
        [NSColor.whiteColor setStroke];[cross stroke];
    }
    NSDictionary *face=frame[@"face"];
    if ([face isKindOfClass:NSDictionary.class]) {
        for (NSString *key in @[@"left_eye",@"right_eye"]) {
            NSArray *points=face[key]; if (!points.count)continue;
            NSBezierPath *eye=[NSBezierPath bezierPath];eye.lineWidth=1.4;
            [eye moveToPoint:[self point:points[0] in:rect]];
            for (NSDictionary *point in points) [eye lineToPoint:[self point:point in:rect]];
            [eye closePath];[NSColor.systemYellowColor setStroke];[eye stroke];
        }
        for (NSString *key in @[@"left_pupil",@"right_pupil"]) if ([face[key] isKindOfClass:NSDictionary.class]) {
            NSPoint p=[self point:face[key] in:rect];[NSColor.systemPinkColor setFill];[[NSBezierPath bezierPathWithOvalInRect:NSMakeRect(p.x-3,p.y-3,6,6)] fill];
        }
    }
    NSArray *origin=self.evaluation[@"grab_origin"],*center=self.evaluation[@"center"];
    if ([origin isKindOfClass:NSArray.class] && [center isKindOfClass:NSArray.class]) {
        NSBezierPath *drag=[NSBezierPath bezierPath];drag.lineWidth=3;[drag moveToPoint:[self pair:origin in:rect]];[drag lineToPoint:[self pair:center in:rect]];
        [NSColor.systemGreenColor setStroke];[drag stroke];
    }
}
@end
@interface GGPreview : NSView
@property(nonatomic,strong) AVCaptureVideoPreviewLayer *video;
@property(nonatomic,strong) GGOverlay *overlay;
@end
@implementation GGPreview
- (instancetype)initWithFrame:(NSRect)rect {
    self=[super initWithFrame:rect];if(!self)return nil;
    self.wantsLayer=YES;self.layer.backgroundColor=NSColor.blackColor.CGColor;self.layer.cornerRadius=8;self.layer.masksToBounds=YES;
    self.video=[AVCaptureVideoPreviewLayer new];self.video.hidden=YES;self.video.videoGravity=AVLayerVideoGravityResizeAspect;[self.layer addSublayer:self.video];
    self.overlay=[[GGOverlay alloc] initWithFrame:self.bounds];self.overlay.wantsLayer=YES;[self addSubview:self.overlay];return self;
}
- (void)layout {[super layout];self.video.frame=self.bounds;self.overlay.frame=self.bounds;}
@end
@interface GGEyePlot : NSView
@property(nonatomic,strong) NSDictionary *signal;
@end
@implementation GGEyePlot
- (void)drawRect:(NSRect)dirty {
    (void)dirty;[NSColor.controlBackgroundColor setFill];[[NSBezierPath bezierPathWithRoundedRect:self.bounds xRadius:8 yRadius:8] fill];
    NSDictionary *attributes=@{NSFontAttributeName:[NSFont systemFontOfSize:10],NSForegroundColorAttributeName:NSColor.secondaryLabelColor};
    [@"Oční signál · nekalibrovaný" drawAtPoint:NSMakePoint(10,self.bounds.size.height-19) withAttributes:attributes];
    NSRect plot=NSInsetRect(self.bounds,12,14);plot.size.height-=18;
    NSBezierPath *grid=[NSBezierPath bezierPath];[grid moveToPoint:NSMakePoint(NSMidX(plot),NSMinY(plot))];[grid lineToPoint:NSMakePoint(NSMidX(plot),NSMaxY(plot))];
    [grid moveToPoint:NSMakePoint(NSMinX(plot),NSMidY(plot))];[grid lineToPoint:NSMakePoint(NSMaxX(plot),NSMidY(plot))];[NSColor.separatorColor setStroke];[grid stroke];
    if ([self.signal isKindOfClass:NSDictionary.class]) {
        double x=[self.signal[@"x"] doubleValue],y=[self.signal[@"y"] doubleValue];NSPoint p=NSMakePoint(plot.origin.x+(1-x)*plot.size.width,plot.origin.y+y*plot.size.height);
        [NSColor.controlAccentColor setFill];[[NSBezierPath bezierPathWithOvalInRect:NSMakeRect(p.x-5,p.y-5,10,10)] fill];
    } else [@"Čekám na obě otevřené oči" drawAtPoint:NSMakePoint(14,NSMidY(plot)-5) withAttributes:attributes];
}
@end
@interface GazeApp : NSObject <NSApplicationDelegate,NSWindowDelegate,AVCaptureVideoDataOutputSampleBufferDelegate>
@property(nonatomic,strong) NSStatusItem *status;
@property(nonatomic,strong) NSPanel *window;
@property(nonatomic,strong) GGPreview *preview;
@property(nonatomic,strong) GGEyePlot *eyePlot;
@property(nonatomic,strong) NSTextField *cameraStatus,*gesture,*handMetrics,*eyeMetrics,*performance,*eventsLabel,*thresholdLabel;
@property(nonatomic,strong) NSButton *cameraButton,*controlButton;
@property(nonatomic,strong) NSPopUpButton *scrollPicker;
@property(nonatomic,strong) NSTextField *controlStatus;
@property(nonatomic,strong) id escapeLocal,escapeGlobal;
@property(nonatomic,strong) NSPanel *guide;
@property(nonatomic,strong) GGPreview *guidePreview;
@property(nonatomic,strong) NSTextField *guideStepLabel,*guideTitle,*guideInstruction,*guideFeedback,*guideMethodLabel,*guideSensitivityLabel;
@property(nonatomic,strong) NSImageView *guideIcon;
@property(nonatomic,strong) NSProgressIndicator *guideProgress;
@property(nonatomic,strong) NSButton *guideBack,*guideNext,*guideSkip,*guideCamera;
@property(nonatomic,strong) NSPopUpButton *guideMethod,*clickMethod;
@property(nonatomic,strong) NSSlider *guideSensitivity,*clickSlider;
@property(nonatomic) NSInteger guideStep,guideClicks;
@property(nonatomic) BOOL guidePassed;
@property(nonatomic) double guideTravel,guideObservedSince;
@property(atomic) BOOL pinchClick;
@property(atomic) double pointerGain;
@property(atomic) BOOL controlEnabled,scrollClutch;
@property(atomic) double clickSensitivity;
@property(nonatomic) double controlsAfter;
@property(nonatomic) CGDirectDisplayID controlDisplay;
@property(nonatomic,strong) NSPopUpButton *cameraPicker;
@property(nonatomic,strong) NSMutableArray<NSString *> *events;
@property(nonatomic,strong) NSArray<AVCaptureDevice *> *devices;
@property(nonatomic,strong) NSTimer *health;
@property(nonatomic,strong) dispatch_queue_t sessionQueue,visionQueue;
@property(atomic,strong) AVCaptureSession *session;
@property(atomic,strong) AVCaptureVideoDataOutput *videoOutput;
@property(atomic) BOOL requested,running;
@property(atomic) NSUInteger generation,outputGeneration;
@property(atomic) double threshold;
@property(nonatomic) double lastVisionTime,lastResultTime;
@property(nonatomic) BOOL staleReported;
@property(nonatomic) void *tracker;
@property(nonatomic,strong) VNDetectHumanHandPoseRequest *handRequest;
@property(nonatomic,strong) VNDetectFaceLandmarksRequest *faceRequest;
@end
@implementation GazeApp
- (NSTextField *)label:(NSString *)text frame:(NSRect)rect {
    NSTextField *label=[NSTextField wrappingLabelWithString:text];label.frame=rect;[self.window.contentView addSubview:label];return label;
}
- (void)applicationDidFinishLaunching:(NSNotification *)note {
    (void)note;self.sessionQueue=dispatch_queue_create("cz.attosoft.gaze.session",DISPATCH_QUEUE_SERIAL);
    self.visionQueue=dispatch_queue_create("cz.attosoft.gaze.vision",DISPATCH_QUEUE_SERIAL);
    self.tracker=gaze_create();self.events=[NSMutableArray new];
    id saved=[NSUserDefaults.standardUserDefaults objectForKey:@"pinchThreshold"];
    self.threshold=[saved isKindOfClass:NSNumber.class] && isfinite([saved doubleValue]) ? MAX(0.15,MIN(0.65,[saved doubleValue])) : 0.35;
    self.clickSensitivity=[NSUserDefaults.standardUserDefaults objectForKey:@"clickSensitivity"] ? [NSUserDefaults.standardUserDefaults doubleForKey:@"clickSensitivity"] : 0.32;
    self.pinchClick=[NSUserDefaults.standardUserDefaults boolForKey:@"pinchClick"];
    self.pointerGain=[NSUserDefaults.standardUserDefaults objectForKey:@"pointerGain"] ? [NSUserDefaults.standardUserDefaults doubleForKey:@"pointerGain"] : 4.;
    self.scrollClutch=[NSUserDefaults.standardUserDefaults boolForKey:@"scrollClutch"];
    self.handRequest=[VNDetectHumanHandPoseRequest new];self.handRequest.maximumHandCount=2;
    self.faceRequest=[VNDetectFaceLandmarksRequest new];
    self.status=[NSStatusBar.systemStatusBar statusItemWithLength:NSSquareStatusItemLength];self.status.autosaveName=@"GazeGrabStatus";
    NSImage *icon=[NSImage imageWithSystemSymbolName:@"eye" accessibilityDescription:@"Gaze Grab"];icon.size=NSMakeSize(18,18);icon.template=YES;self.status.button.image=icon;
    self.status.button.accessibilityLabel=@"Gaze Grab · diagnostika";self.status.button.toolTip=@"Gaze Grab · kamera a gesta";
    [self makeWindow];[self refreshMenu];
    __weak GazeApp *weakSelf=self;
    self.escapeLocal=[NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskKeyDown handler:^NSEvent *(NSEvent *event){if(event.keyCode==53)[weakSelf disableControl];return event;}];
    self.escapeGlobal=[NSEvent addGlobalMonitorForEventsMatchingMask:NSEventMaskKeyDown handler:^(NSEvent *event){if(event.keyCode==53)[weakSelf disableControl];}];
    [NSWorkspace.sharedWorkspace.notificationCenter addObserver:self selector:@selector(suspend:) name:NSWorkspaceSessionDidResignActiveNotification object:nil];
    [NSWorkspace.sharedWorkspace.notificationCenter addObserver:self selector:@selector(suspend:) name:NSWorkspaceWillSleepNotification object:nil];
    self.health=[NSTimer scheduledTimerWithTimeInterval:0.25 target:self selector:@selector(checkHealth:) userInfo:nil repeats:YES];
    if (![NSProcessInfo.processInfo.arguments containsObject:@"--background"]) {[self show:nil];if(![NSUserDefaults.standardUserDefaults boolForKey:@"gestureGuideSeen"])[self showGuide:nil];}
}
- (void)makeWindow {
    self.window=[[NSPanel alloc] initWithContentRect:NSMakeRect(0,0,680,710) styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable|NSWindowStyleMaskMiniaturizable|NSWindowStyleMaskUtilityWindow backing:NSBackingStoreBuffered defer:NO];
    self.window.title=@"Gaze Grab · Diagnostika";self.window.releasedWhenClosed=NO;self.window.delegate=self;self.window.hidesOnDeactivate=NO;self.window.level=NSFloatingWindowLevel;[self.window center];
    [self label:@"Ruce, oči a kurzor" frame:NSMakeRect(16,472,420,24)].font=[NSFont systemFontOfSize:18 weight:NSFontWeightSemibold];
    NSButton *top=[NSButton checkboxWithTitle:@"Vždy nahoře" target:self action:@selector(changeTop:)];top.state=NSControlStateValueOn;top.frame=NSMakeRect(516,470,148,28);[self.window.contentView addSubview:top];
    self.cameraStatus=[self label:@"Kamera je vypnutá. Spusť ji tlačítkem dole." frame:NSMakeRect(16,438,648,24)];self.cameraStatus.textColor=NSColor.secondaryLabelColor;
    self.preview=[[GGPreview alloc] initWithFrame:NSMakeRect(16,184,416,234)];[self.window.contentView addSubview:self.preview];
    self.gesture=[self label:@"Čekám na ruku" frame:NSMakeRect(448,384,216,34)];self.gesture.font=[NSFont systemFontOfSize:16 weight:NSFontWeightSemibold];
    self.handMetrics=[self label:@"Ukaž celou otevřenou ruku.\nPak spoj palec, ukazovák\na prostředník." frame:NSMakeRect(448,292,216,86)];
    self.eyeMetrics=[self label:@"Obličej: —\nZornice: —\nOční signál: —" frame:NSMakeRect(448,208,216,78)];
    self.performance=[self label:@"FPS: —   Analýza: —" frame:NSMakeRect(448,180,216,22)];self.performance.font=[NSFont monospacedDigitSystemFontOfSize:11 weight:NSFontWeightRegular];
    NSSlider *threshold=[NSSlider sliderWithValue:self.threshold minValue:0.15 maxValue:0.65 target:self action:@selector(changeThreshold:)];threshold.frame=NSMakeRect(448,146,150,24);threshold.accessibilityLabel=@"Práh sevření vůči šířce dlaně";[self.window.contentView addSubview:threshold];
    self.thresholdLabel=[self label:@"Sevření < 0,35 · puštění > 0,56" frame:NSMakeRect(448,124,216,20)];self.thresholdLabel.font=[NSFont systemFontOfSize:10];
    self.thresholdLabel.stringValue=[NSString stringWithFormat:@"Sevření < %.2f · puštění > %.2f",self.threshold,self.threshold*1.6];
    self.eventsLabel=[self label:@"Události gest se zobrazí zde." frame:NSMakeRect(448,52,216,66)];self.eventsLabel.font=[NSFont monospacedSystemFontOfSize:10 weight:NSFontWeightRegular];
    self.eyePlot=[[GGEyePlot alloc] initWithFrame:NSMakeRect(16,72,220,100)];[self.window.contentView addSubview:self.eyePlot];
    NSTextField *hint=[self label:@"Tyrkysová: pravá ruka.\nFialová: levá ruka.\nSevření tří prstů funguje\nnezávisle pro obě ruce.\nObraz se zpracovává lokálně." frame:NSMakeRect(248,68,184,104)];hint.font=[NSFont systemFontOfSize:11];hint.textColor=NSColor.secondaryLabelColor;
    self.cameraButton=[NSButton buttonWithTitle:@"Zapnout kameru" target:self action:@selector(startStop:)];self.cameraButton.frame=NSMakeRect(16,18,148,32);[self.window.contentView addSubview:self.cameraButton];
    self.cameraPicker=[[NSPopUpButton alloc] initWithFrame:NSMakeRect(176,18,256,32) pullsDown:NO];self.cameraPicker.accessibilityLabel=@"Kamera";[self.window.contentView addSubview:self.cameraPicker];
    for (NSView *view in self.window.contentView.subviews) { NSRect f=view.frame; f.origin.y+=150;view.frame=f; }
    self.controlButton=[NSButton checkboxWithTitle:@"Ovládat systémový kurzor (Esc vypne)" target:self action:@selector(toggleControl:)];
    self.controlButton.frame=NSMakeRect(16,114,400,26);[self.window.contentView addSubview:self.controlButton];
    self.controlStatus=[self label:@"Zkušební režim · gesta se jen vyhodnocují." frame:NSMakeRect(16,82,648,28)];self.controlStatus.font=[NSFont systemFontOfSize:11];
    [self label:@"Scroll:" frame:NSMakeRect(16,48,48,24)];
    self.scrollPicker=[[NSPopUpButton alloc] initWithFrame:NSMakeRect(68,44,345,28) pullsDown:NO];
    [self.scrollPicker addItemsWithTitles:@[@"Jedna ruka · ukazovák + prostředník",@"Obě ruce · levá drží sevření, pravá scrolluje"]];
    [self.scrollPicker selectItemAtIndex:self.scrollClutch ? 1 : 0];self.scrollPicker.target=self;self.scrollPicker.action=@selector(changeScroll:);self.scrollPicker.accessibilityLabel=@"Gesto pro scrollování";[self.window.contentView addSubview:self.scrollPicker];
    [self label:@"Citlivost kliku:" frame:NSMakeRect(430,48,105,24)];
    NSSlider *click=[NSSlider sliderWithValue:0.75-self.clickSensitivity minValue:0.20 maxValue:0.55 target:self action:@selector(changeClick:)];click.frame=NSMakeRect(535,46,125,26);click.accessibilityLabel=@"Citlivost kliku směrem ke kameře";[self.window.contentView addSubview:click];
    [self label:@"Pohyb stačí malý. Pomalý je přesný, rychlý přesune kurzor dál. Gesta vysvětlí průvodce." frame:NSMakeRect(16,10,648,26)].font=[NSFont systemFontOfSize:11];
    NSButton *guide=[NSButton buttonWithTitle:@"Naučit se gesta…" target:self action:@selector(showGuide:)];guide.frame=NSMakeRect(330,618,175,28);[self.window.contentView addSubview:guide];
    // Make room for the click method and relative movement controls below.
    for(NSView *view in self.window.contentView.subviews){NSRect f=view.frame;f.origin.y+=50;view.frame=f;}
    [self label:@"Klik:" frame:NSMakeRect(16,16,40,24)];
    self.clickMethod=[[NSPopUpButton alloc] initWithFrame:NSMakeRect(60,12,285,28) pullsDown:NO];[self.clickMethod addItemsWithTitles:@[@"Zatlačení ukazováku ke kameře",@"Spojení palce a ukazováku"]];[self.clickMethod selectItemAtIndex:self.pinchClick ? 1 : 0];self.clickMethod.target=self;self.clickMethod.action=@selector(changeClickMethod:);self.clickMethod.accessibilityLabel=@"Způsob kliknutí";[self.window.contentView addSubview:self.clickMethod];
    [self label:@"Rychlost kurzoru:" frame:NSMakeRect(362,16,126,24)];
    NSSlider *gain=[NSSlider sliderWithValue:self.pointerGain minValue:1 maxValue:8 target:self action:@selector(changePointerGain:)];gain.frame=NSMakeRect(495,14,165,26);gain.accessibilityLabel=@"Citlivost relativního pohybu kurzoru";[self.window.contentView addSubview:gain];
    self.clickSlider=click;click.enabled=!self.pinchClick;
    [self reloadCameras];
}
- (void)reloadCameras {
    NSMutableArray *types=[NSMutableArray arrayWithObject:AVCaptureDeviceTypeBuiltInWideAngleCamera];
    if (@available(macOS 14.0,*)) { [types addObject:AVCaptureDeviceTypeExternal];[types addObject:AVCaptureDeviceTypeContinuityCamera]; }
    else [types addObject:AVCaptureDeviceTypeExternalUnknown];
    self.devices=[AVCaptureDeviceDiscoverySession discoverySessionWithDeviceTypes:types mediaType:AVMediaTypeVideo position:AVCaptureDevicePositionUnspecified].devices;
    NSString *selected=self.cameraPicker.selectedItem.representedObject;
    [self.cameraPicker removeAllItems];
    for (AVCaptureDevice *device in self.devices) {
        [self.cameraPicker addItemWithTitle:device.localizedName];self.cameraPicker.lastItem.representedObject=device.uniqueID;
        if ([device.uniqueID isEqual:selected]) [self.cameraPicker selectItem:self.cameraPicker.lastItem];
    }
    if (!self.devices.count) { [self.cameraPicker addItemWithTitle:@"Žádná kamera není dostupná"];self.cameraPicker.enabled=NO; }
}
- (void)refreshMenu {
    NSMenu *menu=[NSMenu new];
    NSMenuItem *heading=[[NSMenuItem alloc] initWithTitle:self.running ? @"Gaze Grab · Kamera běží" : @"Gaze Grab · Kamera vypnutá" action:nil keyEquivalent:@""];[menu addItem:heading];
    for (NSArray *entry in @[@[@"Diagnostika…",@"show:"],@[self.requested ? @"Vypnout kameru" : @"Zapnout kameru",@"startStop:"],@[self.controlEnabled ? @"Vypnout ovládání kurzoru" : @"Zapnout ovládání kurzoru…",@"toggleControl:"],@[@"Vynulovat gesta",@"reset:"],@[@"Povolení kamery v macOS…",@"privacy:"]]) {
        NSMenuItem *item=[[NSMenuItem alloc] initWithTitle:entry[0] action:NSSelectorFromString(entry[1]) keyEquivalent:@""];item.target=self;[menu addItem:item];
    }
    [menu addItem:NSMenuItem.separatorItem];NSMenuItem *quit=[[NSMenuItem alloc] initWithTitle:@"Ukončit Gaze Grab" action:@selector(quit:) keyEquivalent:@""];quit.target=self;[menu addItem:quit];self.status.menu=menu;
}
- (void)show:(id)sender {(void)sender;[NSApp activateIgnoringOtherApps:YES];[self.window makeKeyAndOrderFront:nil];}
- (void)changeTop:(NSButton *)sender {self.window.level=sender.state==NSControlStateValueOn ? NSFloatingWindowLevel : NSNormalWindowLevel;}
- (void)changeThreshold:(NSSlider *)sender {
    self.threshold=sender.doubleValue;[NSUserDefaults.standardUserDefaults setDouble:self.threshold forKey:@"pinchThreshold"];self.thresholdLabel.stringValue=[NSString stringWithFormat:@"Sevření < %.2f · puštění > %.2f",self.threshold,self.threshold*1.6];
    [self reset:nil];
}
- (void)reset:(id)sender {
    (void)sender;dispatch_async(self.visionQueue, ^{gaze_destroy(self.tracker);self.tracker=gaze_create();});
    [self.events removeAllObjects];self.eventsLabel.stringValue=@"Gesta vynulována. Ukaž otevřenou ruku.";
}
- (void)changeScroll:(NSPopUpButton *)sender {self.scrollClutch=sender.indexOfSelectedItem==1;[NSUserDefaults.standardUserDefaults setBool:self.scrollClutch forKey:@"scrollClutch"];[self reset:nil];}
- (void)changeClick:(NSSlider *)sender {self.clickSensitivity=0.75-sender.doubleValue;self.clickSlider.doubleValue=sender.doubleValue;self.guideSensitivity.doubleValue=sender.doubleValue;[NSUserDefaults.standardUserDefaults setDouble:self.clickSensitivity forKey:@"clickSensitivity"];[self reset:nil];}
- (void)changePointerGain:(NSSlider *)sender {self.pointerGain=sender.doubleValue;[NSUserDefaults.standardUserDefaults setDouble:self.pointerGain forKey:@"pointerGain"];}
- (void)changeClickMethod:(NSPopUpButton *)sender {
    self.pinchClick=sender.indexOfSelectedItem==1;[NSUserDefaults.standardUserDefaults setBool:self.pinchClick forKey:@"pinchClick"];
    [self.clickMethod selectItemAtIndex:self.pinchClick ? 1 : 0];[self.guideMethod selectItemAtIndex:self.pinchClick ? 1 : 0];self.clickSlider.enabled=!self.pinchClick;
    if(self.guide.visible)[self configureGuideStep];else [self reset:nil];
}
#include "guide.inc"
- (void)disableControl {
    self.controlEnabled=NO;self.controlButton.state=NSControlStateValueOff;self.controlStatus.stringValue=@"Zkušební režim · gesta se jen vyhodnocují.";[self refreshMenu];
}
- (void)toggleControl:(id)sender {
    (void)sender;if(self.guide.visible){[self disableControl];[self.guide makeKeyAndOrderFront:nil];return;}if(self.controlEnabled){[self disableControl];return;}
    if(!self.running){[self statusText:@"Nejprve zapni kameru, pak povol ovládání kurzoru."];self.controlButton.state=NSControlStateValueOff;return;}
    if(!AXIsProcessTrustedWithOptions((__bridge CFDictionaryRef)@{(__bridge NSString *)kAXTrustedCheckOptionPrompt:@YES})) {
        self.controlButton.state=NSControlStateValueOff;self.controlStatus.stringValue=@"Povol Gaze Grab v macOS → Soukromí a zabezpečení → Zpřístupnění, pak přepínač zapni znovu.";
        return;
    }
    // Register key monitoring after Accessibility is available as well.
    if(self.escapeGlobal)[NSEvent removeMonitor:self.escapeGlobal];
    __weak GazeApp *weakSelf=self;
    self.escapeGlobal=[NSEvent addGlobalMonitorForEventsMatchingMask:NSEventMaskKeyDown handler:^(NSEvent *event){if(event.keyCode==53)[weakSelf disableControl];}];
    self.controlEnabled=YES;self.controlButton.state=NSControlStateValueOn;self.controlsAfter=NSProcessInfo.processInfo.systemUptime+2.;
    self.controlDisplay=CGMainDisplayID();[self reset:nil];[self refreshMenu];
}
- (void)applyControl:(NSDictionary *)control frameTime:(double)time {
    if(!self.controlEnabled || self.guide.visible)return;
    double now=NSProcessInfo.processInfo.systemUptime;
    if(!AXIsProcessTrusted()){[self disableControl];return;}
    if(now<self.controlsAfter){self.controlStatus.stringValue=@"Ovládání začne za 2 s · připrav pravou ruku. Esc vypne.";return;}
    if(now-time>0.25)return;
    self.controlStatus.stringValue=[NSString stringWithFormat:@"OVLÁDÁNÍ · hlavní monitor · %@ · Esc vypne",control[@"reason"] ?: @"čekám"];
    NSArray *movement=control[@"delta"];
    if([movement isKindOfClass:NSArray.class] && movement.count==2) {
        CGRect screen=CGDisplayBounds(self.controlDisplay);double x=[movement[0] doubleValue],y=[movement[1] doubleValue];
        if(isfinite(x)&&isfinite(y) && (x!=0 || y!=0)) {
            CGEventRef current=CGEventCreate(NULL);
            if(current){CGPoint old=CGEventGetLocation(current);CFRelease(current);
                CGPoint position=CGPointMake(MAX(CGRectGetMinX(screen),MIN(CGRectGetMaxX(screen)-1,old.x+x*screen.size.width)),MAX(CGRectGetMinY(screen),MIN(CGRectGetMaxY(screen)-1,old.y+y*screen.size.height)));
                CGEventRef move=CGEventCreateMouseEvent(NULL,kCGEventMouseMoved,position,kCGMouseButtonLeft);if(move){CGEventPost(kCGHIDEventTap,move);CFRelease(move);}
            }
        }
    }
    if([control[@"click"] boolValue]) {
        CGEventRef current=CGEventCreate(NULL);if(current){CGPoint position=CGEventGetLocation(current);CFRelease(current);
            for(int i=0;i<2;i++){CGEventRef click=CGEventCreateMouseEvent(NULL,i==0 ? kCGEventLeftMouseDown : kCGEventLeftMouseUp,position,kCGMouseButtonLeft);if(click){CGEventSetIntegerValueField(click,kCGMouseEventClickState,1);CGEventPost(kCGHIDEventTap,click);CFRelease(click);}}
        }
    }
    double delta=[control[@"scroll"] doubleValue];
    if(isfinite(delta) && fabs(delta)>=1){CGEventRef scroll=CGEventCreateScrollWheelEvent(NULL,kCGScrollEventUnitPixel,1,(int32_t)round(delta));if(scroll){CGEventPost(kCGHIDEventTap,scroll);CFRelease(scroll);}}
}
- (void)privacy:(id)sender {(void)sender;[NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:@"x-apple.systempreferences:com.apple.preference.security?Privacy_Camera"]];}
- (void)startStop:(id)sender {
    (void)sender;if(self.requested){[self stop];return;}
    [self reloadCameras];if(!self.devices.count){[self statusText:@"Mac nehlásí žádnou dostupnou kameru. Připoj ji a zkus to znovu."];return;}
    AVAuthorizationStatus authorization=[AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo];
    if (authorization==AVAuthorizationStatusDenied || authorization==AVAuthorizationStatusRestricted) {[self statusText:@"Povol Gaze Grab v macOS → Soukromí a zabezpečení → Kamera."];[self privacy:nil];return;}
    self.requested=YES;NSUInteger epoch=++self.generation;self.cameraPicker.enabled=NO;self.cameraButton.title=@"Vypnout kameru";[self refreshMenu];
    if (authorization==AVAuthorizationStatusNotDetermined) {
        [self statusText:@"Čekám na povolení kamery v macOS…"];
        [AVCaptureDevice requestAccessForMediaType:AVMediaTypeVideo completionHandler:^(BOOL granted) {
            dispatch_async(dispatch_get_main_queue(), ^{if(epoch!=self.generation || !self.requested)return;if(granted)[self configure:epoch];else{[self stop];[self statusText:@"Kamera nebyla povolena. Přístup změníš v nastavení macOS."];}});
        }];
    } else [self configure:epoch];
}
- (void)statusText:(NSString *)text {self.cameraStatus.stringValue=text;self.cameraStatus.toolTip=text;}
- (void)configure:(NSUInteger)epoch {
    NSString *deviceID=self.cameraPicker.selectedItem.representedObject;[self statusText:@"Spouštím kameru a Apple Vision…"];[self reset:nil];
    dispatch_async(self.sessionQueue, ^{
        if(epoch!=self.generation || !self.requested)return;
        AVCaptureDevice *device=[AVCaptureDevice deviceWithUniqueID:deviceID];NSError *error=nil;
        AVCaptureDeviceInput *input=device ? [AVCaptureDeviceInput deviceInputWithDevice:device error:&error] : nil;
        if(!input){[self fail:error.localizedDescription ?: @"Vybraná kamera není dostupná." epoch:epoch];return;}
        AVCaptureSession *session=[AVCaptureSession new];[session beginConfiguration];
        session.sessionPreset=[session canSetSessionPreset:AVCaptureSessionPreset1280x720] ? AVCaptureSessionPreset1280x720 : AVCaptureSessionPreset640x480;
        AVCaptureVideoDataOutput *output=[AVCaptureVideoDataOutput new];output.alwaysDiscardsLateVideoFrames=YES;
        output.videoSettings=@{(id)kCVPixelBufferPixelFormatTypeKey:@(kCVPixelFormatType_32BGRA)};
        if(![session canAddInput:input]){[session commitConfiguration];[self fail:@"Kameru nelze připojit. Zkontroluj, zda je dostupná." epoch:epoch];return;}[session addInput:input];
        if(![session canAddOutput:output]){[session commitConfiguration];[self fail:@"Výstup kamery není dostupný." epoch:epoch];return;}[session addOutput:output];
        AVCaptureConnection *connection=[output connectionWithMediaType:AVMediaTypeVideo];
        if(connection.isVideoMirroringSupported){connection.automaticallyAdjustsVideoMirroring=NO;connection.videoMirrored=NO;}
        [output setSampleBufferDelegate:self queue:self.visionQueue];[session commitConfiguration];
        self.session=session;self.videoOutput=output;self.outputGeneration=epoch;
        if(epoch!=self.generation || !self.requested){[output setSampleBufferDelegate:nil queue:NULL];return;}
        [NSNotificationCenter.defaultCenter addObserver:self selector:@selector(runtimeError:) name:AVCaptureSessionRuntimeErrorNotification object:session];
        [session startRunning];
        dispatch_async(dispatch_get_main_queue(), ^{
            if(epoch!=self.generation || !self.requested)return;
            self.running=session.running;self.lastResultTime=NSProcessInfo.processInfo.systemUptime;self.staleReported=NO;
            self.preview.video.session=session;self.preview.video.hidden=!self.running;
            if(self.preview.video.connection.isVideoMirroringSupported){self.preview.video.connection.automaticallyAdjustsVideoMirroring=NO;self.preview.video.connection.videoMirrored=YES;}
            [self statusText:self.running ? @"Kamera běží · ukaž jednu nebo obě ruce a obě oči. Náhled je zrcadlený." : @"Kamera neposkytuje obraz. Zkus ji znovu spustit."];
            [self attachGuideCamera];[self refreshMenu];
        });
    });
}
- (void)fail:(NSString *)message epoch:(NSUInteger)epoch {
    dispatch_async(dispatch_get_main_queue(), ^{if(epoch!=self.generation)return;[self stop];[self statusText:message];});
}
- (void)stop {
    [self disableControl];self.requested=NO;self.running=NO;self.generation++;self.cameraButton.title=@"Zapnout kameru";self.cameraPicker.enabled=self.devices.count>0;
    self.preview.video.session=nil;self.preview.video.hidden=YES;self.preview.overlay.landmarksFrame=nil;self.preview.overlay.evaluation=nil;self.preview.overlay.needsDisplay=YES;
    self.eyePlot.signal=nil;self.eyePlot.needsDisplay=YES;self.gesture.stringValue=@"Kamera vypnutá";
    self.handMetrics.stringValue=@"Uchopení zrušeno.\nPro další pokus zapni kameru.";self.eyeMetrics.stringValue=@"Obličej: —\nZornice: —\nOční signál: —";self.performance.stringValue=@"FPS: —   Analýza: —";
    self.guidePreview.video.session=nil;self.guidePreview.video.hidden=YES;[self updateGuide:nil evaluation:@{}];
    [self statusText:@"Kamera je vypnutá. Obraz se neukládá ani neodesílá."];
    dispatch_async(self.sessionQueue, ^{
        AVCaptureSession *session=self.session;AVCaptureVideoDataOutput *output=self.videoOutput;
        [NSNotificationCenter.defaultCenter removeObserver:self name:AVCaptureSessionRuntimeErrorNotification object:session];
        [output setSampleBufferDelegate:nil queue:NULL];[session stopRunning];self.session=nil;self.videoOutput=nil;
    });
    dispatch_async(self.visionQueue, ^{gaze_destroy(self.tracker);self.tracker=gaze_create();self.lastVisionTime=0;});
    [self refreshMenu];
}
- (void)captureOutput:(AVCaptureOutput *)output didOutputSampleBuffer:(CMSampleBufferRef)sample fromConnection:(AVCaptureConnection *)connection {
    (void)connection;NSUInteger epoch=self.outputGeneration;
    if(!self.requested || output!=self.videoOutput || epoch!=self.generation)return;
    double now=NSProcessInfo.processInfo.systemUptime;if(now-self.lastVisionTime<1./15.)return;
    double interval=self.lastVisionTime ? now-self.lastVisionTime : 0;self.lastVisionTime=now;
    @autoreleasepool {
        CVPixelBufferRef buffer=CMSampleBufferGetImageBuffer(sample);if(!buffer)return;
        NSError *error=nil;VNImageRequestHandler *handler=[[VNImageRequestHandler alloc] initWithCVPixelBuffer:buffer orientation:kCGImagePropertyOrientationUp options:@{}];
        if(![handler performRequests:@[self.handRequest,self.faceRequest] error:&error]){[self fail:error.localizedDescription ?: @"Apple Vision nedokončil analýzu." epoch:epoch];return;}
        NSArray *hands=self.handRequest.results,*faces=self.faceRequest.results;
        NSMutableDictionary *frame=[@{@"time":@(now),@"width":@(CVPixelBufferGetWidth(buffer)),@"height":@(CVPixelBufferGetHeight(buffer)),@"threshold":@(self.threshold),@"hand":NSNull.null,@"face":NSNull.null} mutableCopy];
        NSMutableArray *observed=[NSMutableArray new];
        for(VNHumanHandPoseObservation *hand in hands){
            NSMutableArray *points=[NSMutableArray new];
            for(NSString *joint in JointNames()){VNRecognizedPoint *point=[hand recognizedPointForJointName:joint error:nil];[points addObject:PointJSON(point ? point.location : CGPointZero,point ? point.confidence : 0)];}
            NSString *side=hand.chirality==VNChiralityRight ? @"right" : hand.chirality==VNChiralityLeft ? @"left" : @"unknown";
            [observed addObject:@{@"side":side,@"points":points}];
        }
        frame[@"click_method"]=self.pinchClick ? @"pinch" : @"push";frame[@"pointer_gain"]=@(self.pointerGain);
        frame[@"hands"]=observed;frame[@"scroll_clutch"]=@(self.scrollClutch);frame[@"sensitivity"]=@(self.clickSensitivity);
        VNFaceObservation *face=faces.count==1 ? faces[0] : nil;VNFaceLandmarks2D *landmarks=face.landmarks;
        if(landmarks){
            float confidence=MIN(face.confidence,landmarks.confidence);CGRect box=face.boundingBox;
            NSArray *left=FacePoints(landmarks.leftPupil,box,confidence),*right=FacePoints(landmarks.rightPupil,box,confidence);
            frame[@"face"]=@{@"confidence":@(confidence),@"left_eye":FacePoints(landmarks.leftEye,box,confidence),@"right_eye":FacePoints(landmarks.rightEye,box,confidence),@"left_pupil":left.firstObject ?: NSNull.null,@"right_pupil":right.firstObject ?: NSNull.null};
        }
        NSDictionary *evaluation=Evaluate(self.tracker,frame);double latency=(NSProcessInfo.processInfo.systemUptime-now)*1000;
        NSString *pose=face ? [NSString stringWithFormat:@"Hlava: yaw %@°, pitch %@°",face.yaw ? [NSString stringWithFormat:@"%.0f",face.yaw.doubleValue*180/M_PI] : @"—",face.pitch ? [NSString stringWithFormat:@"%.0f",face.pitch.doubleValue*180/M_PI] : @"—"] : @"Hlava: —";
        dispatch_async(dispatch_get_main_queue(), ^{
            if(epoch!=self.generation || !self.requested)return;
            [self display:frame evaluation:evaluation hands:hands.count faces:faces.count pose:pose latency:latency fps:interval>0 ? 1/interval : 0];
        });
    }
}
- (NSString *)number:(id)value {return [value isKindOfClass:NSNumber.class] ? [NSString stringWithFormat:@"%.2f",[value doubleValue]] : @"—";}
- (void)display:(NSDictionary *)frame evaluation:(NSDictionary *)evaluation hands:(NSUInteger)hands faces:(NSUInteger)faces pose:(NSString *)pose latency:(double)latency fps:(double)fps {
    if(self.staleReported)[self statusText:@"Kamera běží · ukaž jednu nebo obě ruce a obě oči. Náhled je zrcadlený."];
    self.lastResultTime=NSProcessInfo.processInfo.systemUptime;self.staleReported=NO;
    self.preview.overlay.landmarksFrame=frame;self.preview.overlay.evaluation=evaluation;self.preview.overlay.needsDisplay=YES;
    NSDictionary *states=@{@"lost":@"Ruku nevidím",@"waiting_open":@"Ukaž otevřenou ruku",@"ready":@"Připraveno",@"arming":@"Potvrzuji sevření",@"held":@"Uchopeno · 3 prsty"};
    NSDictionary *right=evaluation[@"right"],*left=evaluation[@"left"],*control=evaluation[@"control"];
    self.gesture.stringValue=[NSString stringWithFormat:@"P: %@ · L: %@",states[right[@"state"]] ?: @"—",states[left[@"state"]] ?: @"—"];
    self.gesture.font=[NSFont systemFontOfSize:12 weight:NSFontWeightSemibold];
    self.gesture.textColor=([right[@"holding"] boolValue] || [left[@"holding"] boolValue]) ? NSColor.systemGreenColor : NSColor.labelColor;
    self.handMetrics.stringValue=[NSString stringWithFormat:@"Ruce: %lu · P %.0f %% / L %.0f %%\n%@\nUkazovák: %@ · %@",(unsigned long)hands,[right[@"hand_confidence"] doubleValue]*100,[left[@"hand_confidence"] doubleValue]*100,control[@"reason"] ?: @"Čekám",[self number:control[@"extension"]],[control[@"armed"] boolValue] ? @"klik připraven" : @"neaktivní"];
    [self updateGuide:frame evaluation:evaluation];
    [self applyControl:control frameTime:[frame[@"time"] doubleValue]];
    NSDictionary *signal=[evaluation[@"eye_signal"] isKindOfClass:NSDictionary.class] ? evaluation[@"eye_signal"] : nil;
    self.eyePlot.signal=signal;self.eyePlot.needsDisplay=YES;
    self.eyeMetrics.stringValue=[NSString stringWithFormat:@"Obličeje: %lu%@\n%@\nOči: %@\nSignál X/Y: %@ / %@",(unsigned long)faces,faces>1 ? @" · nech jen jeden" : @"",pose,signal ? @"obě otevřené, platné body" : @"body chybí nebo mrknutí",[self number:signal[@"x"]],[self number:signal[@"y"]]];
    self.performance.stringValue=[NSString stringWithFormat:@"%.1f FPS · analýza %.0f ms",fps,latency];
    NSMutableArray *messages=[NSMutableArray new];
    for (NSString *side in @[@"right",@"left"]) {
        NSString *event=evaluation[side][@"event"];
        if([event isKindOfClass:NSString.class])[messages addObject:[NSString stringWithFormat:@"%@ %@",[side isEqual:@"right"] ? @"P" : @"L",event]];
    }
    if([control[@"click"] boolValue])[messages addObject:self.controlEnabled ? @"KLIK" : @"KLIK (náhled)"];
    for (NSString *message in messages) {
        [self.events insertObject:[NSString stringWithFormat:@"%@ %@",[NSDateFormatter localizedStringFromDate:NSDate.date dateStyle:NSDateFormatterNoStyle timeStyle:NSDateFormatterMediumStyle],message] atIndex:0];
        while(self.events.count>4)[self.events removeLastObject];self.eventsLabel.stringValue=[self.events componentsJoinedByString:@"\n"];
    }
}
- (void)checkHealth:(NSTimer *)timer {
    (void)timer;if(self.running && NSProcessInfo.processInfo.systemUptime-self.lastResultTime>1.2 && !self.staleReported){
        [self disableControl];self.staleReported=YES;self.preview.overlay.landmarksFrame=nil;self.preview.overlay.evaluation=nil;self.preview.overlay.needsDisplay=YES;
        self.eyePlot.signal=nil;self.eyePlot.needsDisplay=YES;self.gesture.stringValue=@"Čekám na snímky";
        self.guidePreview.overlay.landmarksFrame=nil;self.guidePreview.overlay.needsDisplay=YES;self.guideFeedback.stringValue=@"Čekám na nové snímky kamery. Gesto zatím nelze ověřit.";
        [self statusText:@"Kamera neposílá nové výsledky. Po obnovení bude nutné znovu otevřít ruku."];
    }
}
- (void)runtimeError:(NSNotification *)note {[self fail:[note.userInfo[AVCaptureSessionErrorKey] localizedDescription] ?: @"Kamera byla přerušena." epoch:self.generation];}
- (void)suspend:(NSNotification *)note {(void)note;[self stop];}
- (void)windowWillClose:(NSNotification *)note {if(note.object==self.guide){[NSUserDefaults.standardUserDefaults setBool:YES forKey:@"gestureGuideSeen"];self.guidePreview.video.session=nil;return;}[self.guide close];[self stop];}
- (BOOL)applicationShouldHandleReopen:(NSApplication *)sender hasVisibleWindows:(BOOL)visible {(void)sender;(void)visible;[self show:nil];return NO;}
- (void)quit:(id)sender {(void)sender;[NSApp terminate:nil];}
- (void)applicationWillTerminate:(NSNotification *)note {
    (void)note;self.requested=NO;self.generation++;[self.health invalidate];
    if(self.escapeLocal)[NSEvent removeMonitor:self.escapeLocal];if(self.escapeGlobal)[NSEvent removeMonitor:self.escapeGlobal];
    [NSWorkspace.sharedWorkspace.notificationCenter removeObserver:self];[NSNotificationCenter.defaultCenter removeObserver:self];
    dispatch_sync(self.sessionQueue, ^{
        [self.videoOutput setSampleBufferDelegate:nil queue:NULL];[self.session stopRunning];self.videoOutput=nil;self.session=nil;
    });
    dispatch_sync(self.visionQueue, ^{gaze_destroy(self.tracker);self.tracker=NULL;});
}
@end
void gaze_run(void){@autoreleasepool{[NSApplication sharedApplication];[NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];GazeApp *app=[GazeApp new];NSApp.delegate=app;[NSApp run];}}
BOOL gaze_vision_probe(void) {
    @autoreleasepool {
        CVPixelBufferRef buffer=NULL;
        if(CVPixelBufferCreate(NULL,640,480,kCVPixelFormatType_32BGRA,NULL,&buffer)!=kCVReturnSuccess)return NO;
        CVPixelBufferLockBaseAddress(buffer,0);memset(CVPixelBufferGetBaseAddress(buffer),0,CVPixelBufferGetBytesPerRow(buffer)*480);CVPixelBufferUnlockBaseAddress(buffer,0);
        VNDetectHumanHandPoseRequest *hands=[VNDetectHumanHandPoseRequest new];VNDetectFaceLandmarksRequest *face=[VNDetectFaceLandmarksRequest new];NSError *error=nil;
        VNImageRequestHandler *handler=[[VNImageRequestHandler alloc] initWithCVPixelBuffer:buffer orientation:kCGImagePropertyOrientationUp options:@{}];
        BOOL ok=[handler performRequests:@[hands,face] error:&error];if(error)NSLog(@"Vision probe: %@",error);CVPixelBufferRelease(buffer);
        return ok && hands.results.count==0 && face.results.count==0;
    }
}

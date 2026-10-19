static NSArray *ActionNames(void) {return @[@"Jen rozpoznávat",@"Klik",@"Dvojklik",@"Pravé tlačítko",@"Scroll nahoru",@"Scroll dolů",@"Pozastavit / obnovit kurzor"];}
static NSArray *ActionIDs(void) {return @[@"none",@"click",@"double_click",@"right_click",@"scroll_up",@"scroll_down",@"pause"];}
- (NSTextField *)learnLabel:(NSString *)text frame:(NSRect)rect {
    NSTextField *label=[NSTextField wrappingLabelWithString:text];label.frame=rect;[self.learning.contentView addSubview:label];return label;
}
- (void)customCommand:(NSDictionary *)command completion:(void (^)(NSDictionary *))completion {
    dispatch_async(self.visionQueue, ^{
        NSDictionary *result=Command(self.tracker,command);
        dispatch_async(dispatch_get_main_queue(), ^{if(completion)completion(result);});
    });
}
- (void)showLearning:(id)sender {
    (void)sender;[self.guide close];[self disableControl];
    if(!self.learning){
        self.learning=[[NSPanel alloc] initWithContentRect:NSMakeRect(0,0,720,650) styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable backing:NSBackingStoreBuffered defer:NO];self.learning.title=@"Gaze Grab · Vlastní gesta";self.learning.releasedWhenClosed=NO;self.learning.delegate=self;self.learning.hidesOnDeactivate=NO;self.learning.level=NSFloatingWindowLevel;
        [self learnLabel:@"Vlastní gesta" frame:NSMakeRect(24,604,670,30)].font=[NSFont systemFontOfSize:21 weight:NSFontWeightSemibold];
        [self learnLabel:@"Uložené gesto" frame:NSMakeRect(24,580,340,20)].font=[NSFont systemFontOfSize:11];
        self.learnSaved=[[NSPopUpButton alloc] initWithFrame:NSMakeRect(20,548,350,28) pullsDown:NO];self.learnSaved.target=self;self.learnSaved.action=@selector(selectLearned:);self.learnSaved.accessibilityLabel=@"Uložená vlastní gesta";[self.learning.contentView addSubview:self.learnSaved];
        self.learnAssigned=[[NSPopUpButton alloc] initWithFrame:NSMakeRect(386,548,310,28) pullsDown:NO];[self.learnAssigned addItemsWithTitles:ActionNames()];self.learnAssigned.accessibilityLabel=@"Akce uloženého gesta";[self.learning.contentView addSubview:self.learnAssigned];
        self.learnEnabled=[NSButton checkboxWithTitle:@"Rozpoznávat" target:nil action:nil];self.learnEnabled.frame=NSMakeRect(24,512,145,28);[self.learning.contentView addSubview:self.learnEnabled];
        self.learnAssign=[NSButton buttonWithTitle:@"Uložit přiřazení" target:self action:@selector(assignLearned:)];self.learnAssign.frame=NSMakeRect(178,510,160,32);[self.learning.contentView addSubview:self.learnAssign];
        self.learnDelete=[NSButton buttonWithTitle:@"Smazat gesto" target:self action:@selector(deleteLearned:)];self.learnDelete.frame=NSMakeRect(542,510,154,32);[self.learning.contentView addSubview:self.learnDelete];
        [self learnLabel:@"Naučit nové gesto ze tří ukázek" frame:NSMakeRect(24,474,670,26)].font=[NSFont systemFontOfSize:16 weight:NSFontWeightSemibold];
        self.learnName=[[NSTextField alloc] initWithFrame:NSMakeRect(24,438,252,28)];self.learnName.placeholderString=@"Název gesta";self.learnName.accessibilityLabel=@"Název nového gesta";[self.learning.contentView addSubview:self.learnName];
        self.learnHands=[[NSPopUpButton alloc] initWithFrame:NSMakeRect(286,436,180,30) pullsDown:NO];[self.learnHands addItemsWithTitles:@[@"Pravá ruka",@"Levá ruka",@"Obě ruce"]];self.learnHands.accessibilityLabel=@"Ruce pro učení";[self.learning.contentView addSubview:self.learnHands];
        self.learnKind=[[NSPopUpButton alloc] initWithFrame:NSMakeRect(476,436,220,30) pullsDown:NO];[self.learnKind addItemsWithTitles:@[@"Póza (podržet)",@"Pohyb (zopakovat)"]];self.learnKind.accessibilityLabel=@"Druh učeného gesta";[self.learning.contentView addSubview:self.learnKind];
        self.learnPreview=[[GGPreview alloc] initWithFrame:NSMakeRect(24,190,350,230)];[self.learning.contentView addSubview:self.learnPreview];
        [self learnLabel:@"Akce nového gesta" frame:NSMakeRect(394,400,300,20)].font=[NSFont systemFontOfSize:11];
        self.learnAction=[[NSPopUpButton alloc] initWithFrame:NSMakeRect(390,368,306,30) pullsDown:NO];[self.learnAction addItemsWithTitles:ActionNames()];self.learnAction.accessibilityLabel=@"Akce nového gesta";[self.learning.contentView addSubview:self.learnAction];
        self.learnInstruction=[self learnLabel:@"Pózu drž dvě sekundy. Pohyb za dvě sekundy zopakuj od začátku do konce. Před každou ukázkou máš dvě sekundy na přípravu." frame:NSMakeRect(394,266,300,94)];
        self.learnFeedback=[self learnLabel:@"Zadej název, vyber ruce a stiskni Připravit gesto." frame:NSMakeRect(394,206,300,54)];self.learnFeedback.font=[NSFont systemFontOfSize:12 weight:NSFontWeightMedium];
        self.learnProgress=[[NSProgressIndicator alloc] initWithFrame:NSMakeRect(394,190,300,10)];self.learnProgress.indeterminate=NO;self.learnProgress.maxValue=1;self.learnProgress.accessibilityLabel=@"Průběh nahrávání ukázky";[self.learning.contentView addSubview:self.learnProgress];
        self.learnBegin=[NSButton buttonWithTitle:@"Připravit gesto" target:self action:@selector(beginLearning:)];self.learnBegin.frame=NSMakeRect(20,138,164,32);[self.learning.contentView addSubview:self.learnBegin];
        self.learnRecord=[NSButton buttonWithTitle:@"Nahrát ukázku 1/3" target:self action:@selector(recordLearning:)];self.learnRecord.frame=NSMakeRect(190,138,164,32);self.learnRecord.enabled=NO;[self.learning.contentView addSubview:self.learnRecord];
        self.learnSave=[NSButton buttonWithTitle:@"Uložit gesto" target:self action:@selector(saveLearning:)];self.learnSave.frame=NSMakeRect(360,138,154,32);self.learnSave.enabled=NO;[self.learning.contentView addSubview:self.learnSave];
        self.learnCancel=[NSButton buttonWithTitle:@"Začít znovu" target:self action:@selector(cancelLearning:)];self.learnCancel.frame=NSMakeRect(524,138,172,32);[self.learning.contentView addSubview:self.learnCancel];
        self.learnStatus=[self learnLabel:@"Zkoušení tu jen ukazuje rozpoznání. Akce do ostatních aplikací se při otevřeném učení neposílají." frame:NSMakeRect(24,70,672,58)];
        self.learnCamera=[NSButton buttonWithTitle:@"Zapnout kameru" target:self action:@selector(startStop:)];self.learnCamera.frame=NSMakeRect(20,24,164,32);[self.learning.contentView addSubview:self.learnCamera];
        [self learnLabel:@"Ukládají se jen body rukou, nikoli video. Esc vypíná ovládání." frame:NSMakeRect(202,26,492,26)].font=[NSFont systemFontOfSize:11];[self.learning center];
    }
    [self.learning makeKeyAndOrderFront:nil];[NSApp activateIgnoringOtherApps:YES];[self attachLearningCamera];[self reloadLearned];
}
- (void)attachLearningCamera {
    if(!self.learning.visible)return;self.learnPreview.video.session=self.session;self.learnPreview.video.hidden=!self.running;
    AVCaptureConnection *connection=self.learnPreview.video.connection;if(connection.isVideoMirroringSupported){connection.automaticallyAdjustsVideoMirroring=NO;connection.videoMirrored=YES;}
    self.learnCamera.title=self.requested ? @"Vypnout kameru" : @"Zapnout kameru";
}
- (void)reloadLearned {
    [self customCommand:@{@"command":@"list"} completion:^(NSDictionary *result){
        id selected=self.learnSaved.selectedItem.representedObject;self.learnModels=result[@"gestures"] ?: @[];[self.learnSaved removeAllItems];
        for(NSDictionary *g in self.learnModels){[self.learnSaved addItemWithTitle:g[@"name"]];self.learnSaved.lastItem.representedObject=g[@"id"];if([selected isEqual:g[@"id"]])[self.learnSaved selectItem:self.learnSaved.lastItem];}
        if(!self.learnModels.count)[self.learnSaved addItemWithTitle:@"Zatím žádné naučené gesto"];
        [self selectLearned:nil];if([result[@"message"] length])self.learnStatus.stringValue=result[@"message"];
    }];
}
- (NSDictionary *)selectedLearned {id identifier=self.learnSaved.selectedItem.representedObject;for(NSDictionary *g in self.learnModels)if([g[@"id"] isEqual:identifier])return g;return nil;}
- (void)selectLearned:(id)sender {
    (void)sender;NSDictionary *g=[self selectedLearned];self.learnAssigned.enabled=g!=nil;self.learnEnabled.enabled=g!=nil;self.learnAssign.enabled=g!=nil;self.learnDelete.enabled=g!=nil;
    if(g){NSUInteger index=[ActionIDs() indexOfObject:g[@"action"]];[self.learnAssigned selectItemAtIndex:index==NSNotFound ? 0 : index];self.learnEnabled.state=[g[@"enabled"] boolValue] ? NSControlStateValueOn : NSControlStateValueOff;}
}
- (void)assignLearned:(id)sender {
    (void)sender;NSDictionary *g=[self selectedLearned];if(!g)return;
    [self customCommand:@{@"command":@"assign",@"id":g[@"id"],@"action":ActionIDs()[self.learnAssigned.indexOfSelectedItem],@"enabled":@(self.learnEnabled.state==NSControlStateValueOn)} completion:^(NSDictionary *r){self.learnStatus.stringValue=r[@"message"] ?: @"";[self reloadLearned];}];
}
- (void)deleteLearned:(id)sender {
    (void)sender;NSDictionary *g=[self selectedLearned];if(!g)return;
    [self customCommand:@{@"command":@"delete",@"id":g[@"id"]} completion:^(NSDictionary *r){self.learnStatus.stringValue=r[@"message"] ?: @"";[self reloadLearned];}];
}
- (void)beginLearning:(id)sender {
    (void)sender;if(!self.running){self.learnStatus.stringValue=@"Nejdřív zapni kameru tlačítkem dole.";return;}
    NSString *hands=@[@"right",@"left",@"both"][self.learnHands.indexOfSelectedItem];NSString *kind=self.learnKind.indexOfSelectedItem==0 ? @"pose" : @"motion";
    [self customCommand:@{@"command":@"begin",@"name":self.learnName.stringValue,@"hands":hands,@"kind":kind,@"action":ActionIDs()[self.learnAction.indexOfSelectedItem]} completion:^(NSDictionary *r){self.learnStatus.stringValue=r[@"message"] ?: @"";if([r[@"ok"] boolValue]){self.learnDraft=YES;[self setLearningFields:NO];self.learnRecord.enabled=YES;self.learnSave.enabled=NO;}}];
}
- (void)setLearningFields:(BOOL)enabled {self.learnName.enabled=enabled;self.learnHands.enabled=enabled;self.learnKind.enabled=enabled;self.learnAction.enabled=enabled;self.learnBegin.enabled=enabled;}
- (void)recordLearning:(id)sender {
    (void)sender;if(!self.running)return;self.learnRecord.enabled=NO;
    [self customCommand:@{@"command":@"record"} completion:^(NSDictionary *r){self.learnStatus.stringValue=r[@"message"] ?: @"";if(![r[@"ok"] boolValue])self.learnRecord.enabled=YES;}];
}
- (void)saveLearning:(id)sender {
    (void)sender;[self customCommand:@{@"command":@"save"} completion:^(NSDictionary *r){self.learnStatus.stringValue=r[@"message"] ?: @"";if([r[@"ok"] boolValue]){self.learnDraft=NO;[self setLearningFields:YES];self.learnRecord.enabled=NO;self.learnSave.enabled=NO;[self reloadLearned];}}];
}
- (void)cancelLearning:(id)sender {
    (void)sender;[self customCommand:@{@"command":@"cancel"} completion:^(NSDictionary *r){self.learnStatus.stringValue=r[@"message"] ?: @"";self.learnDraft=NO;[self setLearningFields:YES];self.learnRecord.enabled=NO;self.learnSave.enabled=NO;self.learnProgress.doubleValue=0;self.learnFeedback.stringValue=@"Zadej název, vyber ruce a připrav gesto.";}];
}
- (void)updateLearning:(NSDictionary *)frame evaluation:(NSDictionary *)evaluation {
    NSDictionary *learned=evaluation[@"learned"];
    if(self.learning.visible){
        self.learnPreview.overlay.landmarksFrame=frame;self.learnPreview.overlay.evaluation=evaluation;self.learnPreview.overlay.needsDisplay=YES;
        self.learnFeedback.stringValue=learned[@"message"] ?: @"Čekám na snímky kamery";self.learnProgress.doubleValue=[learned[@"progress"] doubleValue];
        if([learned[@"training"] boolValue]){NSInteger count=[learned[@"samples"] integerValue];self.learnRecord.title=[NSString stringWithFormat:@"Nahrát ukázku %ld/3",(long)MIN(3,count+1)];self.learnRecord.enabled=self.running && ![learned[@"busy"] boolValue] && count<3;self.learnSave.enabled=count==3;}
        self.learnCamera.title=self.requested ? @"Vypnout kameru" : @"Zapnout kameru";
    }
    NSDictionary *event=[learned[@"event"] isKindOfClass:NSDictionary.class] ? learned[@"event"] : nil;
    if(event){[self.events insertObject:[NSString stringWithFormat:@"Gesto: %@",event[@"name"]] atIndex:0];while(self.events.count>4)[self.events removeLastObject];self.eventsLabel.stringValue=[self.events componentsJoinedByString:@"\n"];}
    if(!event || !self.controlEnabled || self.guide.visible || self.learning.visible || !AXIsProcessTrusted() || NSProcessInfo.processInfo.systemUptime<self.controlsAfter || NSProcessInfo.processInfo.systemUptime-[frame[@"time"] doubleValue]>0.25)return;
    NSString *action=event[@"action"];if([action isEqual:@"none"])return;
    self.pendingCursor=CGPointZero;
    if([action isEqual:@"pause"]){self.gesturePaused=!self.gesturePaused;self.controlStatus.stringValue=self.gesturePaused ? @"Kurzor pozastaven gestem. Zopakuj gesto pro obnovení." : @"Kurzor obnoven.";return;}
    if(self.gesturePaused)return;
    if([action isEqual:@"scroll_up"] || [action isEqual:@"scroll_down"]){CGEventRef scroll=CGEventCreateScrollWheelEvent(NULL,kCGScrollEventUnitPixel,1,[action isEqual:@"scroll_up"] ? 120 : -120);if(scroll){CGEventPost(kCGHIDEventTap,scroll);CFRelease(scroll);}return;}
    if([action isEqual:@"click"] || [action isEqual:@"double_click"] || [action isEqual:@"right_click"]){
        CGEventRef current=CGEventCreate(NULL);if(!current)return;CGPoint position=CGEventGetLocation(current);CFRelease(current);
        BOOL right=[action isEqual:@"right_click"];int count=[action isEqual:@"double_click"] ? 2 : 1;
        for(int click=1;click<=count;click++)for(int phase=0;phase<2;phase++){
            CGEventType type=right ? (phase==0 ? kCGEventRightMouseDown : kCGEventRightMouseUp) : (phase==0 ? kCGEventLeftMouseDown : kCGEventLeftMouseUp);
            CGEventRef e=CGEventCreateMouseEvent(NULL,type,position,right ? kCGMouseButtonRight : kCGMouseButtonLeft);if(e){CGEventSetIntegerValueField(e,kCGMouseEventClickState,click);CGEventPost(kCGHIDEventTap,e);CFRelease(e);}
        }
    }
}

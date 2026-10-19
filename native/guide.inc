// Nonmodal practice window. Uses the same camera results and gesture engine;
// system events remain disabled throughout practice.
- (NSTextField *)guideLabel:(NSString *)text frame:(NSRect)rect {
    NSTextField *label=[NSTextField wrappingLabelWithString:text];label.frame=rect;[self.guide.contentView addSubview:label];return label;
}
- (void)showGuide:(id)sender {
    (void)sender;[self disableControl];
    if(!self.guide) {
        self.guide=[[NSPanel alloc] initWithContentRect:NSMakeRect(0,0,660,550) styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable backing:NSBackingStoreBuffered defer:NO];
        self.guide.title=@"Gaze Grab · Naučit se gesta";self.guide.releasedWhenClosed=NO;self.guide.delegate=self;self.guide.level=NSFloatingWindowLevel;self.guide.hidesOnDeactivate=NO;
        [self guideLabel:@"Vyzkoušej si gesta" frame:NSMakeRect(24,502,610,28)].font=[NSFont systemFontOfSize:21 weight:NSFontWeightSemibold];
        self.guideStepLabel=[self guideLabel:@"" frame:NSMakeRect(24,470,610,22)];self.guideStepLabel.textColor=NSColor.secondaryLabelColor;
        self.guidePreview=[[GGPreview alloc] initWithFrame:NSMakeRect(24,224,330,230)];[self.guide.contentView addSubview:self.guidePreview];
        self.guideIcon=[[NSImageView alloc] initWithFrame:NSMakeRect(380,392,48,48)];self.guideIcon.imageScaling=NSImageScaleProportionallyUpOrDown;self.guideIcon.contentTintColor=NSColor.controlAccentColor;[self.guide.contentView addSubview:self.guideIcon];
        self.guideTitle=[self guideLabel:@"" frame:NSMakeRect(380,342,252,44)];self.guideTitle.font=[NSFont systemFontOfSize:18 weight:NSFontWeightSemibold];
        self.guideInstruction=[self guideLabel:@"" frame:NSMakeRect(380,226,252,108)];self.guideInstruction.font=[NSFont systemFontOfSize:13];
        self.guideFeedback=[self guideLabel:@"" frame:NSMakeRect(24,160,610,52)];self.guideFeedback.font=[NSFont systemFontOfSize:13 weight:NSFontWeightMedium];
        self.guideProgress=[[NSProgressIndicator alloc] initWithFrame:NSMakeRect(24,141,610,10)];self.guideProgress.indeterminate=NO;self.guideProgress.minValue=0;self.guideProgress.maxValue=1;self.guideProgress.accessibilityLabel=@"Průběh zkoušeného gesta";[self.guide.contentView addSubview:self.guideProgress];
        self.guideMethodLabel=[self guideLabel:@"Způsob kliku" frame:NSMakeRect(24,112,310,20)];self.guideMethodLabel.font=[NSFont systemFontOfSize:11];
        self.guideMethod=[[NSPopUpButton alloc] initWithFrame:NSMakeRect(20,80,350,28) pullsDown:NO];[self.guideMethod addItemsWithTitles:@[@"Zatlačení ukazováku ke kameře",@"Spojení palce a ukazováku (snazší)"]];self.guideMethod.target=self;self.guideMethod.action=@selector(changeClickMethod:);self.guideMethod.accessibilityLabel=@"Způsob kliknutí v průvodci";[self.guide.contentView addSubview:self.guideMethod];
        self.guideSensitivityLabel=[self guideLabel:@"Citlivost kliku" frame:NSMakeRect(390,112,242,20)];self.guideSensitivityLabel.font=[NSFont systemFontOfSize:11];
        self.guideSensitivity=[NSSlider sliderWithValue:0.75-self.clickSensitivity minValue:0.20 maxValue:0.55 target:self action:@selector(changeClick:)];self.guideSensitivity.frame=NSMakeRect(390,82,242,26);self.guideSensitivity.accessibilityLabel=@"Citlivost kliku v průvodci";[self.guide.contentView addSubview:self.guideSensitivity];
        self.guideBack=[NSButton buttonWithTitle:@"Zpět" target:self action:@selector(guideBack:)];self.guideBack.frame=NSMakeRect(20,24,76,32);[self.guide.contentView addSubview:self.guideBack];
        self.guideCamera=[NSButton buttonWithTitle:@"Zapnout kameru" target:self action:@selector(startStop:)];self.guideCamera.frame=NSMakeRect(108,24,148,32);[self.guide.contentView addSubview:self.guideCamera];
        self.guideSkip=[NSButton buttonWithTitle:@"Přeskočit" target:self action:@selector(guideNext:)];self.guideSkip.frame=NSMakeRect(388,24,108,32);[self.guide.contentView addSubview:self.guideSkip];
        self.guideNext=[NSButton buttonWithTitle:@"Další gesto" target:self action:@selector(guideNext:)];self.guideNext.frame=NSMakeRect(500,24,136,32);[self.guide.contentView addSubview:self.guideNext];
        [self.guide center];
    }
    [self.guideMethod selectItemAtIndex:self.pinchClick ? 1 : 0];self.guideSensitivity.doubleValue=0.75-self.clickSensitivity;
    self.guideStep=0;[self configureGuideStep];[NSApp activateIgnoringOtherApps:YES];[self.guide makeKeyAndOrderFront:nil];[self attachGuideCamera];
}
- (void)attachGuideCamera {
    if(!self.guide.visible)return;
    self.guidePreview.video.session=self.session;self.guidePreview.video.hidden=!self.running;
    AVCaptureConnection *connection=self.guidePreview.video.connection;
    if(connection.isVideoMirroringSupported){connection.automaticallyAdjustsVideoMirroring=NO;connection.videoMirrored=YES;}
}
- (void)configureGuideStep {
    self.guidePassed=NO;self.guideClicks=0;self.guideTravel=0;self.guideObservedSince=0;self.guideProgress.doubleValue=0;self.guideBack.enabled=self.guideStep>0;self.guideNext.enabled=NO;
    NSArray *titles=@[@"Ukaž pravou ruku",@"Malý pohyb, velký dosah",@"Zkus třikrát kliknout",@"Posuň obsah",@"Zapoj levou ruku"];
    NSArray *symbols=@[@"hand.raised",@"cursorarrow.motionlines",@"hand.point.up",@"hand.point.up.left",@"hands.clap"];
    self.guideStepLabel.stringValue=[NSString stringWithFormat:@"%ld z 5 · Zkoušení neposílá kliky do ostatních aplikací",(long)self.guideStep+1];
    self.guideTitle.stringValue=titles[self.guideStep];self.guideIcon.image=[NSImage imageWithSystemSymbolName:symbols[self.guideStep] accessibilityDescription:titles[self.guideStep]] ?: [NSImage imageWithSystemSymbolName:@"hand.raised" accessibilityDescription:titles[self.guideStep]];
    self.guideInstruction.stringValue=@[@"Nech pravou ruku tam, kde je ti pohodlně. Ukaž dlaň i prsty kameře. Tyrkysová kostra označuje pravou ruku.",@"Pohni rukou jen o pár centimetrů do strany a zpět. Bílý kříž je cvičný kurzor. Pomalu pro přesnost, rychleji pro větší přesun.",self.pinchClick ? @"Nejdřív palec a ukazovák oddal. Až uvidíš Připraveno, spoj jejich špičky a zase je oddal. Opakuj třikrát." : @"Natáhni jen ukazovák a ostatní prsty ohni. Počkej na Připraveno. Dlaň nech stát, ukazovák zatlač ke kameře, potom jej vrať. Opakuj třikrát.",self.scrollClutch ? @"Levou rukou spoj palec, ukazovák a prostředník. Pravou natáhni ukazovák a posuň ho nahoru a dolů." : @"Na pravé ruce natáhni ukazovák a prostředník jako V. Prsteník a malík ohni. Pak pohni prsty nahoru a dolů.",@"Nejdřív ukaž obě otevřené ruce. Na levé spoj palec, ukazovák a prostředník. Pravou přitom malinko pohni. Každá ruka má vlastní gesto."][self.guideStep];
    self.guideMethodLabel.hidden=self.guideStep!=2;self.guideSensitivityLabel.hidden=self.guideStep!=2;self.guideSensitivity.hidden=self.guideStep!=2;
    self.guideMethod.hidden=self.guideStep!=2;self.guideSensitivity.enabled=self.guideStep==2 && !self.pinchClick;
    self.guideNext.title=self.guideStep==4 ? @"Dokončit" : @"Další gesto";self.guideFeedback.textColor=NSColor.labelColor;
    self.guideFeedback.stringValue=self.requested ? @"Čekám na gesto. Sleduj živou odezvu tady." : @"Nejprve zapni kameru tlačítkem dole.";
    [self reset:nil];
}
- (void)guideBack:(id)sender {(void)sender;if(self.guideStep>0){self.guideStep--;[self configureGuideStep];}}
- (void)guideNext:(id)sender {
    (void)sender;if(self.guideStep<4){self.guideStep++;[self configureGuideStep];return;}
    [NSUserDefaults.standardUserDefaults setBool:YES forKey:@"gestureGuideSeen"];[self.guide close];
}
- (void)updateGuide:(NSDictionary *)frame evaluation:(NSDictionary *)evaluation {
    if(!self.guide.visible)return;
    self.guideCamera.title=self.requested ? @"Vypnout kameru" : @"Zapnout kameru";
    self.guidePreview.overlay.landmarksFrame=frame;self.guidePreview.overlay.evaluation=evaluation;self.guidePreview.overlay.needsDisplay=YES;
    if(!self.running){self.guideFeedback.stringValue=@"Zapni kameru tlačítkem dole. Systémový kurzor je při zkoušení vypnutý.";return;}
    if(self.guidePassed)return;
    NSDictionary *control=evaluation[@"control"],*click=control[@"click_feedback"];
    BOOL visible=[control[@"mode"] isEqual:@"pointer"] || [control[@"mode"] isEqual:@"scroll"];
    if(!visible){self.guideObservedSince=0;self.guideFeedback.stringValue=control[@"reason"] ?: @"Ukaž celou pravou ruku v obraze kamery.";self.guideFeedback.textColor=NSColor.labelColor;return;}
    double now=[frame[@"time"] doubleValue];
    if(!self.guideObservedSince)self.guideObservedSince=now;
    double dx=[control[@"delta"][0] doubleValue],dy=[control[@"delta"][1] doubleValue];
    double progress=0;NSString *feedback=@"";
    switch(self.guideStep){
        case 0:progress=MIN(1,(now-self.guideObservedSince)/0.5);feedback=@"Pravou ruku vidím. Ještě ji chvilku podrž.";break;
        case 1:self.guideTravel+=hypot(dx,dy);progress=MIN(1,self.guideTravel/0.35);feedback=@"Pohni rukou trochu do strany a zpět. Sleduj bílý kříž.";break;
        case 2:
            if([control[@"click"] boolValue])self.guideClicks++;
            progress=MIN(1,self.guideClicks/3.);
            feedback=[NSString stringWithFormat:@"%ld ze 3 kliků · %@\nPrůběh aktuálního stisku: %.0f %%",(long)self.guideClicks,control[@"reason"] ?: @"Ukaž prsty",[click[@"progress"] doubleValue]*100];
            break;
        case 3:self.guideTravel+=fabs([control[@"scroll"] doubleValue]);progress=MIN(1,self.guideTravel/100.);feedback=[control[@"mode"] isEqual:@"scroll"] ? @"Scroll rozpoznán. Pokračuj nahoru nebo dolů." : @"Připrav prsty podle obrázku a pokynů vpravo.";break;
        default:
            if([evaluation[@"left"][@"holding"] boolValue])self.guideTravel+=hypot(dx,dy)+fabs([control[@"scroll"] doubleValue])/1000.;
            progress=MIN(1,self.guideTravel/0.12);
            feedback=[evaluation[@"left"][@"holding"] boolValue] ? @"Levá drží sevření. Teď trochu pohni pravou rukou." : @"Ukaž otevřenou levou ruku, pak spoj palec, ukazovák a prostředník.";break;
    }
    self.guideProgress.doubleValue=progress;self.guideFeedback.stringValue=feedback;
    if(progress>=1){self.guidePassed=YES;self.guideNext.enabled=YES;self.guideFeedback.textColor=NSColor.systemGreenColor;self.guideFeedback.stringValue=self.guideStep==2 ? @"Tři kliky rozpoznány. Gesto funguje v náhledu." : @"Gesto rozpoznáno. Můžeš pokračovat.";}
}

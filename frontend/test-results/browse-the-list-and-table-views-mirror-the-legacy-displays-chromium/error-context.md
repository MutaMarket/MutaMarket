# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: browse.spec.ts >> the list and table views mirror the legacy displays
- Location: e2e/browse.spec.ts:38:1

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: getByText('Please select a category')
Expected: visible
Timeout: 1500ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 1500ms
  - waiting for getByText('Please select a category')


Call Log:
- Test timeout of 30000ms exceeded
```

# Page snapshot

```yaml
- generic [ref=f7e2]:
  - banner [ref=f7e3]:
    - generic [ref=f7e6]:
      - link [ref=f7e7] [cursor=pointer]:
        - /url: /
      - navigation [ref=f7e11]:
        - link "Buy" [ref=f7e12] [cursor=pointer]:
          - /url: /
        - link "Appraise" [ref=f7e16] [cursor=pointer]:
          - /url: /modules/add
        - link "Characters" [ref=f7e18] [cursor=pointer]:
          - /url: /characters
        - link "Collections" [ref=f7e24] [cursor=pointer]:
          - /url: /collections
        - button "More" [ref=f7e29] [cursor=pointer]
      - link "Log in" [ref=f7e32] [cursor=pointer]:
        - /url: /login
  - main [ref=f7e34]:
    - generic [ref=f7e35]:
      - generic [ref=f7e36]:
        - generic [ref=f7e41]:
          - heading "All Modules" [level=1] [ref=f7e42]
          - paragraph [ref=f7e43]: The archive · every module ever indexed
        - generic [ref=f7e45]:
          - generic [ref=f7e46]:
            - term [ref=f7e47]: Archived
            - definition [ref=f7e48]: 1,764,424
          - generic [ref=f7e49]:
            - term [ref=f7e50]: Gold bars
            - definition [ref=f7e51]: 1,569
          - generic [ref=f7e52]:
            - term [ref=f7e53]: Diamond bars
            - definition [ref=f7e54]: "28"
          - generic [ref=f7e55]:
            - term [ref=f7e56]: Added 24h
            - definition [ref=f7e57]: 11,755
      - generic [ref=f7e59]:
        - generic [ref=f7e60]:
          - generic [ref=f7e61]:
            - generic [ref=f7e62]:
              - heading "Category" [level=2] [ref=f7e63]
              - button "All" [ref=f7e64] [cursor=pointer]
            - generic [ref=f7e68]:
              - heading "Meta group" [level=2] [ref=f7e69]
              - button "All" [ref=f7e70] [cursor=pointer]
            - generic [ref=f7e71]:
              - heading "Meta level" [level=2] [ref=f7e72]
              - button "All" [ref=f7e73] [cursor=pointer]
          - generic [ref=f7e74]:
            - button "Gold bars" [ref=f7e75] [cursor=pointer]
            - button "Brown bars" [ref=f7e77] [cursor=pointer]
            - button "Diamond bars" [ref=f7e79] [cursor=pointer]
        - generic [ref=f7e82]:
          - generic [ref=f7e83]:
            - heading "Est. value" [level=2] [ref=f7e84]
            - generic [ref=f7e89]:
              - spinbutton "value lower bound" [ref=f7e91]: "1000000"
              - spinbutton "value upper bound" [ref=f7e93]: "100000000000"
            - generic [ref=f7e96] [cursor=pointer]:
              - generic: 1M
              - generic: 10M
              - generic: 100M
              - generic: 1B
              - generic: 10B
              - generic: 100B
              - slider [ref=f7e98]
              - slider [ref=f7e99]
          - generic [ref=f7e100]:
            - button "Sort ascending" [ref=f7e101] [cursor=pointer]
            - generic [ref=f7e102]: Sort
            - button "Sort descending" [ref=f7e103] [cursor=pointer]
      - generic [ref=f7e104]:
        - generic [ref=f7e105]:
          - generic [ref=f7e106]:
            - generic [ref=f7e107]: View
            - generic [ref=f7e108]:
              - button "Grid view" [ref=f7e109] [cursor=pointer]
              - button "List view" [ref=f7e115] [cursor=pointer]
              - button "Table view" [ref=f7e117] [cursor=pointer]
          - generic [ref=f7e120]:
            - generic [ref=f7e121]: Roll bars
            - generic [ref=f7e122]:
              - button "Default" [ref=f7e123] [cursor=pointer]
              - button "Type" [ref=f7e124] [cursor=pointer]
              - button "Absolute" [ref=f7e125] [cursor=pointer]
              - button "None" [ref=f7e126] [cursor=pointer]
          - generic [ref=f7e127]:
            - switch "Scores Scores" [ref=f7e128] [cursor=pointer]
            - generic [ref=f7e129] [cursor=pointer]: Scores
          - generic [ref=f7e130]:
            - generic [ref=f7e131]: Page 1
            - link "Next page" [ref=f7e135] [cursor=pointer]:
              - /url: /all-modules/page/2
        - generic [ref=f7e140]:
          - generic [ref=f7e141]:
            - button [ref=f7e143] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e144]
            - generic [ref=f7e145]:
              - generic [ref=f7e146]:
                - img "capacitorNeed" [ref=f7e147]
                - generic [ref=f7e148]: 264.48GJ
              - generic [ref=f7e154]:
                - img "cpu" [ref=f7e155]
                - generic [ref=f7e156]: 94.6tf
              - generic [ref=f7e162]:
                - img "speedFactor" [ref=f7e163]
                - generic [ref=f7e164]: 539.83%
              - generic [ref=f7e170]:
                - img "power" [ref=f7e171]
                - generic [ref=f7e172]: 1428.63MW
              - generic [ref=f7e178]:
                - img "signatureRadiusBonus" [ref=f7e179]
                - generic [ref=f7e180]: 405.94%
            - generic [ref=f7e187]:
              - link "100 million ISK est. 174 million ISK" [ref=f7e188] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055397504616
                - generic [ref=f7e193]:
                  - generic [ref=f7e194]: 100 million ISK
                  - generic [ref=f7e195]: est. 174 million ISK
              - button [ref=f7e197] [cursor=pointer]
          - generic [ref=f7e198]:
            - button [ref=f7e200] [cursor=pointer]:
              - img "Abyssal Modulated Strip Miner" [ref=f7e201]
            - generic [ref=f7e202]:
              - generic [ref=f7e203]:
                - img "capacitorNeed" [ref=f7e204]
                - generic [ref=f7e205]: 41.33GJ
              - generic [ref=f7e211]:
                - img "duration" [ref=f7e212]
                - generic [ref=f7e213]: 47.24s
              - generic [ref=f7e219]:
                - img "cpu" [ref=f7e220]
                - generic [ref=f7e221]: 57.79tf
              - generic [ref=f7e227]:
                - img "miningCritBonusYield" [ref=f7e228]
                - generic [ref=f7e229]: 227.62%
              - generic [ref=f7e235]:
                - img "miningCritChance" [ref=f7e236]
                - generic [ref=f7e237]: 0.68%
              - generic [ref=f7e243]:
                - img "miningAmount" [ref=f7e244]
                - generic [ref=f7e245]: 141.31m3
              - generic [ref=f7e251]:
                - img "maxRange" [ref=f7e252]
                - generic [ref=f7e253]: 19442.25m
              - generic [ref=f7e259]:
                - img "power" [ref=f7e260]
                - generic [ref=f7e261]: 14.56MW
              - generic [ref=f7e267]:
                - img "miningWasteProbability" [ref=f7e268]
                - generic [ref=f7e269]: 38.75%
              - generic [ref=f7e275]:
                - img "miningWastedVolumeMultiplier" [ref=f7e276]
                - generic [ref=f7e277]: 0.986x
              - generic [ref=f7e283]:
                - img "effectiveMiningSpeed" [ref=f7e284]
                - generic [ref=f7e285]: 3.04m³/s
            - generic [ref=f7e292]:
              - link "132 million ISK est. 174 million ISK" [ref=f7e293] [cursor=pointer]:
                - /url: /modules/abyssal-modulated-strip-miner-1055397502440
                - generic [ref=f7e298]:
                  - generic [ref=f7e299]: 132 million ISK
                  - generic [ref=f7e300]: est. 174 million ISK
              - button [ref=f7e302] [cursor=pointer]
          - generic [ref=f7e303]:
            - button [ref=f7e305] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e306]
            - generic [ref=f7e307]:
              - generic [ref=f7e308]:
                - img "capacitorNeed" [ref=f7e309]
                - generic [ref=f7e310]: 235.63GJ
              - generic [ref=f7e316]:
                - img "cpu" [ref=f7e317]
                - generic [ref=f7e318]: 71.11tf
              - generic [ref=f7e324]:
                - img "speedFactor" [ref=f7e325]
                - generic [ref=f7e326]: 527.28%
              - generic [ref=f7e332]:
                - img "power" [ref=f7e333]
                - generic [ref=f7e334]: 1206.49MW
              - generic [ref=f7e340]:
                - img "signatureRadiusBonus" [ref=f7e341]
                - generic [ref=f7e342]: 513.66%
            - generic [ref=f7e349]:
              - link "100 million ISK est. 209 million ISK" [ref=f7e350] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055397500657
                - generic [ref=f7e355]:
                  - generic [ref=f7e356]: 100 million ISK
                  - generic [ref=f7e357]: est. 209 million ISK
              - button [ref=f7e359] [cursor=pointer]
          - generic [ref=f7e360]:
            - button [ref=f7e362] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e363]
            - generic [ref=f7e364]:
              - generic [ref=f7e365]:
                - img "capacitorNeed" [ref=f7e366]
                - generic [ref=f7e367]: 232.44GJ
              - generic [ref=f7e373]:
                - img "cpu" [ref=f7e374]
                - generic [ref=f7e375]: 70.87tf
              - generic [ref=f7e381]:
                - img "speedFactor" [ref=f7e382]
                - generic [ref=f7e383]: 543.62%
              - generic [ref=f7e389]:
                - img "power" [ref=f7e390]
                - generic [ref=f7e391]: 1207.73MW
              - generic [ref=f7e397]:
                - img "signatureRadiusBonus" [ref=f7e398]
                - generic [ref=f7e399]: 416.76%
            - generic [ref=f7e406]:
              - link "400 million ISK est. 595 million ISK" [ref=f7e407] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055397495738
                - generic [ref=f7e412]:
                  - generic [ref=f7e413]: 400 million ISK
                  - generic [ref=f7e414]: est. 595 million ISK
              - button [ref=f7e416] [cursor=pointer]
          - generic [ref=f7e417]:
            - button [ref=f7e419] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e420]
            - generic [ref=f7e421]:
              - generic [ref=f7e422]:
                - img "capacitorNeed" [ref=f7e423]
                - generic [ref=f7e424]: 235.91GJ
              - generic [ref=f7e430]:
                - img "cpu" [ref=f7e431]
                - generic [ref=f7e432]: 86.77tf
              - generic [ref=f7e438]:
                - img "speedFactor" [ref=f7e439]
                - generic [ref=f7e440]: 542.39%
              - generic [ref=f7e446]:
                - img "power" [ref=f7e447]
                - generic [ref=f7e448]: 1387.79MW
              - generic [ref=f7e454]:
                - img "signatureRadiusBonus" [ref=f7e455]
                - generic [ref=f7e456]: 439.71%
            - generic [ref=f7e463]:
              - link "200 million ISK est. 431 million ISK" [ref=f7e464] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055397490337
                - generic [ref=f7e469]:
                  - generic [ref=f7e470]: 200 million ISK
                  - generic [ref=f7e471]: est. 431 million ISK
              - button [ref=f7e473] [cursor=pointer]
          - generic [ref=f7e474]:
            - button [ref=f7e476] [cursor=pointer]:
              - img "Small Abyssal Energy Nosferatu" [ref=f7e477]
            - generic [ref=f7e478]:
              - generic [ref=f7e479]:
                - img "cpu" [ref=f7e480]
                - generic [ref=f7e481]: 21.6tf
              - generic [ref=f7e487]:
                - img "powerTransferAmount" [ref=f7e488]
                - generic [ref=f7e489]: 11.58points
              - generic [ref=f7e495]:
                - img "maxRange" [ref=f7e496]
                - generic [ref=f7e497]: 9950.4m
              - generic [ref=f7e503]:
                - img "power" [ref=f7e504]
                - generic [ref=f7e505]: 12.3MW
            - generic [ref=f7e512]:
              - link "60 million ISK est. 87 million ISK" [ref=f7e513] [cursor=pointer]:
                - /url: /modules/small-abyssal-energy-nosferatu-1055397484961
                - generic [ref=f7e518]:
                  - generic [ref=f7e519]: 60 million ISK
                  - generic [ref=f7e520]: est. 87 million ISK
              - button [ref=f7e522] [cursor=pointer]
          - generic [ref=f7e523]:
            - button [ref=f7e525] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e526]
            - generic [ref=f7e527]:
              - generic [ref=f7e528]:
                - img "capacitorNeed" [ref=f7e529]
                - generic [ref=f7e530]: 333.5GJ
              - generic [ref=f7e536]:
                - img "cpu" [ref=f7e537]
                - generic [ref=f7e538]: 86.7tf
              - generic [ref=f7e544]:
                - img "speedFactor" [ref=f7e545]
                - generic [ref=f7e546]: 524.1%
              - generic [ref=f7e552]:
                - img "power" [ref=f7e553]
                - generic [ref=f7e554]: 1278.89MW
              - generic [ref=f7e560]:
                - img "signatureRadiusBonus" [ref=f7e561]
                - generic [ref=f7e562]: 423.46%
            - generic [ref=f7e569]:
              - link "20 million ISK est. 32 million ISK" [ref=f7e570] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055397484792
                - generic [ref=f7e575]:
                  - generic [ref=f7e576]: 20 million ISK
                  - generic [ref=f7e577]: est. 32 million ISK
              - button [ref=f7e579] [cursor=pointer]
          - generic [ref=f7e580]:
            - button [ref=f7e582] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e583]
            - generic [ref=f7e584]:
              - generic [ref=f7e585]:
                - img "capacitorNeed" [ref=f7e586]
                - generic [ref=f7e587]: 260.56GJ
              - generic [ref=f7e593]:
                - img "cpu" [ref=f7e594]
                - generic [ref=f7e595]: 97.09tf
              - generic [ref=f7e601]:
                - img "speedFactor" [ref=f7e602]
                - generic [ref=f7e603]: 541.91%
              - generic [ref=f7e609]:
                - img "power" [ref=f7e610]
                - generic [ref=f7e611]: 1277.65MW
              - generic [ref=f7e617]:
                - img "signatureRadiusBonus" [ref=f7e618]
                - generic [ref=f7e619]: 516.66%
            - generic [ref=f7e626]:
              - link "150 million ISK est. 189 million ISK" [ref=f7e627] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055397461798
                - generic [ref=f7e632]:
                  - generic [ref=f7e633]: 150 million ISK
                  - generic [ref=f7e634]: est. 189 million ISK
              - button [ref=f7e636] [cursor=pointer]
          - generic [ref=f7e637]:
            - button [ref=f7e639] [cursor=pointer]:
              - img "Abyssal Warp Scrambler" [ref=f7e640]
            - generic [ref=f7e641]:
              - generic [ref=f7e642]:
                - img "capacitorNeed" [ref=f7e643]
                - generic [ref=f7e644]: 6.7GJ
              - generic [ref=f7e650]:
                - img "cpu" [ref=f7e651]
                - generic [ref=f7e652]: 25.03tf
              - generic [ref=f7e658]:
                - img "maxRange" [ref=f7e659]
                - generic [ref=f7e660]: 10255.5m
            - generic [ref=f7e667]:
              - link "40 million ISK est. 42 million ISK" [ref=f7e668] [cursor=pointer]:
                - /url: /modules/abyssal-warp-scrambler-1055397110614
                - generic [ref=f7e673]:
                  - generic [ref=f7e674]: 40 million ISK
                  - generic [ref=f7e675]: est. 42 million ISK
              - button [ref=f7e677] [cursor=pointer]
          - generic [ref=f7e678]:
            - button [ref=f7e680] [cursor=pointer]:
              - img "Abyssal Warp Scrambler" [ref=f7e681]
            - generic [ref=f7e682]:
              - generic [ref=f7e683]:
                - img "capacitorNeed" [ref=f7e684]
                - generic [ref=f7e685]: 10.24GJ
              - generic [ref=f7e691]:
                - img "cpu" [ref=f7e692]
                - generic [ref=f7e693]: 35.19tf
              - generic [ref=f7e699]:
                - img "maxRange" [ref=f7e700]
                - generic [ref=f7e701]: 12973.5m
            - generic [ref=f7e708]:
              - link "470 million ISK est. 487 million ISK" [ref=f7e709] [cursor=pointer]:
                - /url: /modules/abyssal-warp-scrambler-1055397105603
                - generic [ref=f7e714]:
                  - generic [ref=f7e715]: 470 million ISK
                  - generic [ref=f7e716]: est. 487 million ISK
              - button [ref=f7e718] [cursor=pointer]
          - generic [ref=f7e719]:
            - button [ref=f7e721] [cursor=pointer]:
              - img "Abyssal Warp Scrambler" [ref=f7e722]
            - generic [ref=f7e723]:
              - generic [ref=f7e724]:
                - img "capacitorNeed" [ref=f7e725]
                - generic [ref=f7e726]: 8.51GJ
              - generic [ref=f7e732]:
                - img "cpu" [ref=f7e733]
                - generic [ref=f7e734]: 24.19tf
              - generic [ref=f7e740]:
                - img "maxRange" [ref=f7e741]
                - generic [ref=f7e742]: 10728m
            - generic [ref=f7e749]:
              - link "50 million ISK est. 48 million ISK" [ref=f7e750] [cursor=pointer]:
                - /url: /modules/abyssal-warp-scrambler-1055397104723
                - generic [ref=f7e755]:
                  - generic [ref=f7e756]: 50 million ISK
                  - generic [ref=f7e757]: est. 48 million ISK
              - button [ref=f7e759] [cursor=pointer]
          - generic [ref=f7e760]:
            - button [ref=f7e762] [cursor=pointer]:
              - img "Abyssal Warp Scrambler" [ref=f7e763]
            - generic [ref=f7e764]:
              - generic [ref=f7e765]:
                - img "capacitorNeed" [ref=f7e766]
                - generic [ref=f7e767]: 14.61GJ
              - generic [ref=f7e773]:
                - img "cpu" [ref=f7e774]
                - generic [ref=f7e775]: 25.11tf
              - generic [ref=f7e781]:
                - img "maxRange" [ref=f7e782]
                - generic [ref=f7e783]: 12136.5m
            - generic [ref=f7e790]:
              - link "240 million ISK est. 237 million ISK" [ref=f7e791] [cursor=pointer]:
                - /url: /modules/abyssal-warp-scrambler-1055397103859
                - generic [ref=f7e796]:
                  - generic [ref=f7e797]: 240 million ISK
                  - generic [ref=f7e798]: est. 237 million ISK
              - button [ref=f7e800] [cursor=pointer]
          - generic [ref=f7e801]:
            - button [ref=f7e803] [cursor=pointer]:
              - img "Abyssal Warp Scrambler" [ref=f7e804]
            - generic [ref=f7e805]:
              - generic [ref=f7e806]:
                - img "capacitorNeed" [ref=f7e807]
                - generic [ref=f7e808]: 14.7GJ
              - generic [ref=f7e814]:
                - img "cpu" [ref=f7e815]
                - generic [ref=f7e816]: 26.84tf
              - generic [ref=f7e822]:
                - img "maxRange" [ref=f7e823]
                - generic [ref=f7e824]: 12415.5m
            - generic [ref=f7e831]:
              - link "280 million ISK est. 316 million ISK" [ref=f7e832] [cursor=pointer]:
                - /url: /modules/abyssal-warp-scrambler-1055397099331
                - generic [ref=f7e837]:
                  - generic [ref=f7e838]: 280 million ISK
                  - generic [ref=f7e839]: est. 316 million ISK
              - button [ref=f7e841] [cursor=pointer]
          - generic [ref=f7e842]:
            - button [ref=f7e844] [cursor=pointer]:
              - img "Abyssal Warp Scrambler" [ref=f7e845]
            - generic [ref=f7e846]:
              - generic [ref=f7e847]:
                - img "capacitorNeed" [ref=f7e848]
                - generic [ref=f7e849]: 5.41GJ
              - generic [ref=f7e855]:
                - img "cpu" [ref=f7e856]
                - generic [ref=f7e857]: 35.78tf
              - generic [ref=f7e863]:
                - img "maxRange" [ref=f7e864]
                - generic [ref=f7e865]: 12766.5m
            - generic [ref=f7e872]:
              - link "420 million ISK est. 426 million ISK" [ref=f7e873] [cursor=pointer]:
                - /url: /modules/abyssal-warp-scrambler-1055397098339
                - generic [ref=f7e878]:
                  - generic [ref=f7e879]: 420 million ISK
                  - generic [ref=f7e880]: est. 426 million ISK
              - button [ref=f7e882] [cursor=pointer]
          - generic [ref=f7e883]:
            - button [ref=f7e885] [cursor=pointer]:
              - img "Abyssal Warp Scrambler" [ref=f7e886]
            - generic [ref=f7e887]:
              - generic [ref=f7e888]:
                - img "capacitorNeed" [ref=f7e889]
                - generic [ref=f7e890]: 8.08GJ
              - generic [ref=f7e896]:
                - img "cpu" [ref=f7e897]
                - generic [ref=f7e898]: 26.08tf
              - generic [ref=f7e904]:
                - img "maxRange" [ref=f7e905]
                - generic [ref=f7e906]: 11983.5m
            - generic [ref=f7e913]:
              - link "200 million ISK est. 201 million ISK" [ref=f7e914] [cursor=pointer]:
                - /url: /modules/abyssal-warp-scrambler-1055397096190
                - generic [ref=f7e919]:
                  - generic [ref=f7e920]: 200 million ISK
                  - generic [ref=f7e921]: est. 201 million ISK
              - button [ref=f7e923] [cursor=pointer]
          - generic [ref=f7e924]:
            - button [ref=f7e926] [cursor=pointer]:
              - img "Abyssal Ballistic Control System" [ref=f7e927]
            - generic [ref=f7e928]:
              - generic [ref=f7e929]:
                - img "cpu" [ref=f7e930]
                - generic [ref=f7e931]: 30.74tf
              - generic [ref=f7e937]:
                - img "missileDamageMultiplierBonus" [ref=f7e938]
                - generic [ref=f7e939]: 13.92%
              - generic [ref=f7e945]:
                - img "speedMultiplier" [ref=f7e946]
                - generic [ref=f7e947]: 10.252%
              - generic [ref=f7e953]:
                - img "dpsIncreaseMissiles" [ref=f7e954]
                - generic [ref=f7e955]: 26.93%
            - generic [ref=f7e962]:
              - link "est. 189 million ISK Created by VaLeNokFOX" [ref=f7e963] [cursor=pointer]:
                - /url: /modules/abyssal-ballistic-control-system-1055396963704
                - generic [ref=f7e968]:
                  - generic [ref=f7e969]: est. 189 million ISK
                  - generic [ref=f7e970]: Created by VaLeNokFOX
              - button [ref=f7e972] [cursor=pointer]
          - generic [ref=f7e973]:
            - button [ref=f7e975] [cursor=pointer]:
              - img "Abyssal Ballistic Control System" [ref=f7e976]
            - generic [ref=f7e977]:
              - generic [ref=f7e978]:
                - img "cpu" [ref=f7e979]
                - generic [ref=f7e980]: 31.43tf
              - generic [ref=f7e986]:
                - img "missileDamageMultiplierBonus" [ref=f7e987]
                - generic [ref=f7e988]: 14.15%
              - generic [ref=f7e994]:
                - img "speedMultiplier" [ref=f7e995]
                - generic [ref=f7e996]: 11.596%
              - generic [ref=f7e1002]:
                - img "dpsIncreaseMissiles" [ref=f7e1003]
                - generic [ref=f7e1004]: 29.12%
            - generic [ref=f7e1011]:
              - link "750 million ISK est. 795 million ISK" [ref=f7e1012] [cursor=pointer]:
                - /url: /modules/abyssal-ballistic-control-system-1055396962804
                - generic [ref=f7e1017]:
                  - generic [ref=f7e1018]: 750 million ISK
                  - generic [ref=f7e1019]: est. 795 million ISK
              - button [ref=f7e1021] [cursor=pointer]
          - generic [ref=f7e1022]:
            - button [ref=f7e1024] [cursor=pointer]:
              - img "Abyssal Ballistic Control System" [ref=f7e1025]
            - generic [ref=f7e1026]:
              - generic [ref=f7e1027]:
                - img "cpu" [ref=f7e1028]
                - generic [ref=f7e1029]: 20.53tf
              - generic [ref=f7e1035]:
                - img "missileDamageMultiplierBonus" [ref=f7e1036]
                - generic [ref=f7e1037]: 11.67%
              - generic [ref=f7e1043]:
                - img "speedMultiplier" [ref=f7e1044]
                - generic [ref=f7e1045]: 12.856%
              - generic [ref=f7e1051]:
                - img "dpsIncreaseMissiles" [ref=f7e1052]
                - generic [ref=f7e1053]: 28.15%
            - generic [ref=f7e1060]:
              - link "200 million ISK est. 222 million ISK" [ref=f7e1061] [cursor=pointer]:
                - /url: /modules/abyssal-ballistic-control-system-1055396962758
                - generic [ref=f7e1066]:
                  - generic [ref=f7e1067]: 200 million ISK
                  - generic [ref=f7e1068]: est. 222 million ISK
              - button [ref=f7e1070] [cursor=pointer]
          - generic [ref=f7e1071]:
            - button [ref=f7e1073] [cursor=pointer]:
              - img "Abyssal Ballistic Control System" [ref=f7e1074]
            - generic [ref=f7e1075]:
              - generic [ref=f7e1076]:
                - img "cpu" [ref=f7e1077]
                - generic [ref=f7e1078]: 22.61tf
              - generic [ref=f7e1084]:
                - img "missileDamageMultiplierBonus" [ref=f7e1085]
                - generic [ref=f7e1086]: 10.84%
              - generic [ref=f7e1092]:
                - img "speedMultiplier" [ref=f7e1093]
                - generic [ref=f7e1094]: 12.758%
              - generic [ref=f7e1100]:
                - img "dpsIncreaseMissiles" [ref=f7e1101]
                - generic [ref=f7e1102]: 27.04%
            - generic [ref=f7e1109]:
              - link "100 million ISK est. 137 million ISK" [ref=f7e1110] [cursor=pointer]:
                - /url: /modules/abyssal-ballistic-control-system-1055396961980
                - generic [ref=f7e1115]:
                  - generic [ref=f7e1116]: 100 million ISK
                  - generic [ref=f7e1117]: est. 137 million ISK
              - button [ref=f7e1119] [cursor=pointer]
          - generic [ref=f7e1120]:
            - button [ref=f7e1122] [cursor=pointer]:
              - img "Abyssal Ballistic Control System" [ref=f7e1123]
            - generic [ref=f7e1124]:
              - generic [ref=f7e1125]:
                - img "cpu" [ref=f7e1126]
                - generic [ref=f7e1127]: 29.53tf
              - generic [ref=f7e1133]:
                - img "missileDamageMultiplierBonus" [ref=f7e1134]
                - generic [ref=f7e1135]: 14.09%
              - generic [ref=f7e1141]:
                - img "speedMultiplier" [ref=f7e1142]
                - generic [ref=f7e1143]: 10.666%
              - generic [ref=f7e1149]:
                - img "dpsIncreaseMissiles" [ref=f7e1150]
                - generic [ref=f7e1151]: 27.71%
            - generic [ref=f7e1158]:
              - link "300 million ISK est. 376 million ISK" [ref=f7e1159] [cursor=pointer]:
                - /url: /modules/abyssal-ballistic-control-system-1055396960338
                - generic [ref=f7e1164]:
                  - generic [ref=f7e1165]: 300 million ISK
                  - generic [ref=f7e1166]: est. 376 million ISK
              - button [ref=f7e1168] [cursor=pointer]
          - generic [ref=f7e1169]:
            - button [ref=f7e1171] [cursor=pointer]:
              - img "Abyssal Ballistic Control System" [ref=f7e1172]
            - generic [ref=f7e1173]:
              - generic [ref=f7e1174]:
                - img "cpu" [ref=f7e1175]
                - generic [ref=f7e1176]: 22.91tf
              - generic [ref=f7e1182]:
                - img "missileDamageMultiplierBonus" [ref=f7e1183]
                - generic [ref=f7e1184]: 10.55%
              - generic [ref=f7e1190]:
                - img "speedMultiplier" [ref=f7e1191]
                - generic [ref=f7e1192]: 10.146%
              - generic [ref=f7e1198]:
                - img "dpsIncreaseMissiles" [ref=f7e1199]
                - generic [ref=f7e1200]: 23.03%
            - generic [ref=f7e1207]:
              - link "50 million ISK est. 25 million ISK" [ref=f7e1208] [cursor=pointer]:
                - /url: /modules/abyssal-ballistic-control-system-1055396941018
                - generic [ref=f7e1213]:
                  - generic [ref=f7e1214]: 50 million ISK
                  - generic [ref=f7e1215]: est. 25 million ISK
              - button [ref=f7e1217] [cursor=pointer]
          - generic [ref=f7e1218]:
            - button [ref=f7e1220] [cursor=pointer]:
              - img "Abyssal Magnetic Field Stabilizer" [ref=f7e1221]
            - generic [ref=f7e1222]:
              - generic [ref=f7e1223]:
                - img "cpu" [ref=f7e1224]
                - generic [ref=f7e1225]: 19.32tf
              - generic [ref=f7e1231]:
                - img "damageMultiplier" [ref=f7e1232]
                - generic [ref=f7e1233]: 1.133x
              - generic [ref=f7e1239]:
                - img "speedMultiplier" [ref=f7e1240]
                - generic [ref=f7e1241]: 11.123%
              - generic [ref=f7e1247]:
                - img "dpsIncreaseTurrets" [ref=f7e1248]
                - generic [ref=f7e1249]: 27.46%
            - generic [ref=f7e1256]:
              - link "170 million ISK est. 176 million ISK" [ref=f7e1257] [cursor=pointer]:
                - /url: /modules/abyssal-magnetic-field-stabilizer-1055396856758
                - generic [ref=f7e1262]:
                  - generic [ref=f7e1263]: 170 million ISK
                  - generic [ref=f7e1264]: est. 176 million ISK
              - button [ref=f7e1266] [cursor=pointer]
          - generic [ref=f7e1267]:
            - button [ref=f7e1269] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e1270]
            - generic [ref=f7e1271]:
              - generic [ref=f7e1272]:
                - img "capacitorNeed" [ref=f7e1273]
                - generic [ref=f7e1274]: 170.91GJ
              - generic [ref=f7e1280]:
                - img "cpu" [ref=f7e1281]
                - generic [ref=f7e1282]: 88.83tf
              - generic [ref=f7e1288]:
                - img "speedFactor" [ref=f7e1289]
                - generic [ref=f7e1290]: 557.44%
              - generic [ref=f7e1296]:
                - img "power" [ref=f7e1297]
                - generic [ref=f7e1298]: 1672MW
              - generic [ref=f7e1304]:
                - img "signatureRadiusBonus" [ref=f7e1305]
                - generic [ref=f7e1306]: 502.55%
            - generic [ref=f7e1313]:
              - link "1.6 billion ISK est. 1.8 billion ISK" [ref=f7e1314] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055396575062
                - generic [ref=f7e1319]:
                  - generic [ref=f7e1320]: 1.6 billion ISK
                  - generic [ref=f7e1321]: est. 1.8 billion ISK
              - button [ref=f7e1323] [cursor=pointer]
          - generic [ref=f7e1324]:
            - button [ref=f7e1326] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e1327]
            - generic [ref=f7e1328]:
              - generic [ref=f7e1329]:
                - img "capacitorNeed" [ref=f7e1330]
                - generic [ref=f7e1331]: 173.6GJ
              - generic [ref=f7e1337]:
                - img "cpu" [ref=f7e1338]
                - generic [ref=f7e1339]: 128.34tf
              - generic [ref=f7e1345]:
                - img "speedFactor" [ref=f7e1346]
                - generic [ref=f7e1347]: 561.7%
              - generic [ref=f7e1353]:
                - img "power" [ref=f7e1354]
                - generic [ref=f7e1355]: 1637MW
              - generic [ref=f7e1361]:
                - img "signatureRadiusBonus" [ref=f7e1362]
                - generic [ref=f7e1363]: 426.27%
            - generic [ref=f7e1370]:
              - link "1.7 billion ISK est. 1.7 billion ISK" [ref=f7e1371] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055396573599
                - generic [ref=f7e1376]:
                  - generic [ref=f7e1377]: 1.7 billion ISK
                  - generic [ref=f7e1378]: est. 1.7 billion ISK
              - button [ref=f7e1380] [cursor=pointer]
          - generic [ref=f7e1381]:
            - button [ref=f7e1383] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e1384]
            - generic [ref=f7e1385]:
              - generic [ref=f7e1386]:
                - img "capacitorNeed" [ref=f7e1387]
                - generic [ref=f7e1388]: 260.3GJ
              - generic [ref=f7e1394]:
                - img "cpu" [ref=f7e1395]
                - generic [ref=f7e1396]: 80.77tf
              - generic [ref=f7e1402]:
                - img "speedFactor" [ref=f7e1403]
                - generic [ref=f7e1404]: 521.65%
              - generic [ref=f7e1410]:
                - img "power" [ref=f7e1411]
                - generic [ref=f7e1412]: 1324.4MW
              - generic [ref=f7e1418]:
                - img "signatureRadiusBonus" [ref=f7e1419]
                - generic [ref=f7e1420]: 488.63%
            - generic [ref=f7e1427]:
              - link "est. 38 million ISK Created by VaLeNokFOX" [ref=f7e1428] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055396542805
                - generic [ref=f7e1433]:
                  - generic [ref=f7e1434]: est. 38 million ISK
                  - generic [ref=f7e1435]: Created by VaLeNokFOX
              - button [ref=f7e1437] [cursor=pointer]
          - generic [ref=f7e1438]:
            - button [ref=f7e1440] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e1441]
            - generic [ref=f7e1442]:
              - generic [ref=f7e1443]:
                - img "capacitorNeed" [ref=f7e1444]
                - generic [ref=f7e1445]: 317.65GJ
              - generic [ref=f7e1451]:
                - img "cpu" [ref=f7e1452]
                - generic [ref=f7e1453]: 93.64tf
              - generic [ref=f7e1459]:
                - img "speedFactor" [ref=f7e1460]
                - generic [ref=f7e1461]: 513.2%
              - generic [ref=f7e1467]:
                - img "power" [ref=f7e1468]
                - generic [ref=f7e1469]: 1307.08MW
              - generic [ref=f7e1475]:
                - img "signatureRadiusBonus" [ref=f7e1476]
                - generic [ref=f7e1477]: 434.7%
            - generic [ref=f7e1484]:
              - link "est. 12 million ISK Created by VaLeNokFOX" [ref=f7e1485] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055396542736
                - generic [ref=f7e1490]:
                  - generic [ref=f7e1491]: est. 12 million ISK
                  - generic [ref=f7e1492]: Created by VaLeNokFOX
              - button [ref=f7e1494] [cursor=pointer]
          - generic [ref=f7e1495]:
            - button [ref=f7e1497] [cursor=pointer]:
              - img "500MN Abyssal Microwarpdrive" [ref=f7e1498]
            - generic [ref=f7e1499]:
              - generic [ref=f7e1500]:
                - img "capacitorNeed" [ref=f7e1501]
                - generic [ref=f7e1502]: 257.46GJ
              - generic [ref=f7e1508]:
                - img "cpu" [ref=f7e1509]
                - generic [ref=f7e1510]: 89.72tf
              - generic [ref=f7e1516]:
                - img "speedFactor" [ref=f7e1517]
                - generic [ref=f7e1518]: 525.53%
              - generic [ref=f7e1524]:
                - img "power" [ref=f7e1525]
                - generic [ref=f7e1526]: 1383.8MW
              - generic [ref=f7e1532]:
                - img "signatureRadiusBonus" [ref=f7e1533]
                - generic [ref=f7e1534]: 474.24%
            - generic [ref=f7e1541]:
              - link "est. 111 million ISK Created by VaLeNokFOX" [ref=f7e1542] [cursor=pointer]:
                - /url: /modules/500mn-abyssal-microwarpdrive-1055396539327
                - generic [ref=f7e1547]:
                  - generic [ref=f7e1548]: est. 111 million ISK
                  - generic [ref=f7e1549]: Created by VaLeNokFOX
              - button [ref=f7e1551] [cursor=pointer]
          - generic [ref=f7e1552]:
            - button [ref=f7e1554] [cursor=pointer]:
              - img "100MN Abyssal Afterburner" [ref=f7e1555]
            - generic [ref=f7e1556]:
              - generic [ref=f7e1557]:
                - img "capacitorNeed" [ref=f7e1558]
                - generic [ref=f7e1559]: 319.06GJ
              - generic [ref=f7e1565]:
                - img "cpu" [ref=f7e1566]
                - generic [ref=f7e1567]: 47.97tf
              - generic [ref=f7e1573]:
                - img "speedFactor" [ref=f7e1574]
                - generic [ref=f7e1575]: 141.72%
              - generic [ref=f7e1581]:
                - img "power" [ref=f7e1582]
                - generic [ref=f7e1583]: 798.29MW
            - generic [ref=f7e1590]:
              - link "50 million ISK est. 13 million ISK" [ref=f7e1591] [cursor=pointer]:
                - /url: /modules/100mn-abyssal-afterburner-1055396512333
                - generic [ref=f7e1596]:
                  - generic [ref=f7e1597]: 50 million ISK
                  - generic [ref=f7e1598]: est. 13 million ISK
              - button [ref=f7e1600] [cursor=pointer]
          - generic [ref=f7e1601]:
            - button [ref=f7e1603] [cursor=pointer]:
              - img "Abyssal Warp Scrambler" [ref=f7e1604]
            - generic [ref=f7e1605]:
              - generic [ref=f7e1606]:
                - img "capacitorNeed" [ref=f7e1607]
                - generic [ref=f7e1608]: 10.82GJ
              - generic [ref=f7e1614]:
                - img "cpu" [ref=f7e1615]
                - generic [ref=f7e1616]: 24.61tf
              - generic [ref=f7e1622]:
                - img "maxRange" [ref=f7e1623]
                - generic [ref=f7e1624]: 9594m
            - generic [ref=f7e1631]:
              - link "est. 18 million ISK Created by Colin-x" [ref=f7e1632] [cursor=pointer]:
                - /url: /modules/abyssal-warp-scrambler-1055396349031
                - generic [ref=f7e1637]:
                  - generic [ref=f7e1638]: est. 18 million ISK
                  - generic [ref=f7e1639]: Created by Colin-x
              - button [ref=f7e1641] [cursor=pointer]
          - generic [ref=f7e1642]:
            - button [ref=f7e1644] [cursor=pointer]:
              - img "Abyssal Warp Scrambler" [ref=f7e1645]
            - generic [ref=f7e1646]:
              - generic [ref=f7e1647]:
                - img "capacitorNeed" [ref=f7e1648]
                - generic [ref=f7e1649]: 6.37GJ
              - generic [ref=f7e1655]:
                - img "cpu" [ref=f7e1656]
                - generic [ref=f7e1657]: 37.94tf
              - generic [ref=f7e1663]:
                - img "maxRange" [ref=f7e1664]
                - generic [ref=f7e1665]: 12478.5m
            - generic [ref=f7e1672]:
              - link "210 million ISK est. 304 million ISK" [ref=f7e1673] [cursor=pointer]:
                - /url: /modules/abyssal-warp-scrambler-1055396348018
                - generic [ref=f7e1678]:
                  - generic [ref=f7e1679]: 210 million ISK
                  - generic [ref=f7e1680]: est. 304 million ISK
              - button [ref=f7e1682] [cursor=pointer]
        - generic [ref=f7e1683]:
          - generic [ref=f7e1684]:
            - generic [ref=f7e1685]: View
            - generic [ref=f7e1686]:
              - button "Grid view" [ref=f7e1687] [cursor=pointer]
              - button "List view" [ref=f7e1693] [cursor=pointer]
              - button "Table view" [ref=f7e1695] [cursor=pointer]
          - generic [ref=f7e1698]:
            - generic [ref=f7e1699]: Roll bars
            - generic [ref=f7e1700]:
              - button "Default" [ref=f7e1701] [cursor=pointer]
              - button "Type" [ref=f7e1702] [cursor=pointer]
              - button "Absolute" [ref=f7e1703] [cursor=pointer]
              - button "None" [ref=f7e1704] [cursor=pointer]
          - generic [ref=f7e1705]:
            - switch [ref=f7e1706] [cursor=pointer]
            - generic [ref=f7e1707] [cursor=pointer]: Scores
          - generic [ref=f7e1708]:
            - generic [ref=f7e1709]: Page 1
            - link "Next page" [ref=f7e1713] [cursor=pointer]:
              - /url: /all-modules/page/2
    - generic [ref=f7e1716]:
      - generic [ref=f7e1717]:
        - link [ref=f7e1718] [cursor=pointer]:
          - /url: https://store.eveonline.com
          - img "EVE Store" [ref=f7e1719]
        - generic: Advertisement
      - generic [ref=f7e1720]:
        - generic [ref=f7e1721]: Premium
        - generic [ref=f7e1729]:
          - paragraph [ref=f7e1730]: Unlock historic sales, similar modules, priority ordering, and more.
          - generic [ref=f7e1731]:
            - generic [ref=f7e1732]:
              - generic [ref=f7e1733]: Monthly
              - generic [ref=f7e1734]: 100 million ISK
            - generic [ref=f7e1735]:
              - generic [ref=f7e1736]: Yearly
              - generic [ref=f7e1737]: 1 billion ISK
            - paragraph [ref=f7e1738]: Save 2 months with yearly
        - button "Send ISK to MutaMate" [ref=f7e1740] [cursor=pointer]:
          - generic [ref=f7e1744]: Send ISK to
          - code [ref=f7e1745]: MutaMate
      - link "Buy me some Quafe Help me stay awake and code more" [ref=f7e1749] [cursor=pointer]:
        - /url: https://ko-fi.com/nicolaskion
        - generic [ref=f7e1753]:
          - generic [ref=f7e1754]: Buy me some Quafe
          - generic [ref=f7e1755]: Help me stay awake and code more
      - generic [ref=f7e1756]:
        - generic [ref=f7e1757]: Partner
        - link "WormholeSystems Wormhole mapping & intel" [ref=f7e1762] [cursor=pointer]:
          - /url: https://wormhole.systems
          - generic [ref=f7e1766]:
            - generic [ref=f7e1767]: WormholeSystems
            - generic [ref=f7e1773]: Wormhole mapping & intel
  - contentinfo [ref=f7e1774]:
    - paragraph [ref=f7e1775]: MutaMarket - the marketplace and toolbox for abyssal modules in EVE Online.
  - region "Notifications alt+T"
```

# Test source

```ts
  1   | // The module browser: stats, cards, view switching, filter navigation.
  2   | import { expect, test } from '@playwright/test';
  3   | 
  4   | test('the browser shows the filter band and module cards', async ({ page }) => {
  5   | 	await page.goto('/');
  6   | 	await expect(page.getByRole('heading', { name: 'Modules for Sale' })).toBeVisible();
  7   | 	await expect(page.getByRole('button', { name: 'Only contracts' })).toBeVisible();
  8   | 
  9   | 	// The all-modules page always has cards, independent of whether the
  10  | 	// live market has been swept yet.
  11  | 	await page.goto('/all-modules');
  12  | 	// Scoped to main: the nav's Appraise link also starts with /modules/.
  13  | 	const cards = page.locator('main a[href^="/modules/"]');
  14  | 	await expect(cards.first()).toBeVisible();
  15  | });
  16  | 
  17  | test('filter navigation updates the URL and keeps the browser mounted', async ({ page }) => {
  18  | 	await page.goto('/');
  19  | 	// Retry the click: it can land before hydration and get lost.
  20  | 	await expect(async () => {
  21  | 		await page.getByRole('button', { name: 'Only contracts' }).click();
  22  | 		await expect(page).toHaveURL(/contracts-only/, { timeout: 1000 });
  23  | 	}).toPass();
  24  | 	await expect(page.getByRole('heading', { name: 'Modules for Sale' })).toBeVisible();
  25  | });
  26  | 
  27  | test('a card click opens the module show page', async ({ page }) => {
  28  | 	await page.goto('/all-modules');
  29  | 	const link = page.locator('main a[href^="/modules/"]').first();
  30  | 	const href = await link.getAttribute('href');
  31  | 	await link.click();
  32  | 	await expect(page).toHaveURL(new RegExp(`${href}$`));
  33  | 	// The show page hero and tab strip are up.
  34  | 	await expect(page.getByText('Created by').first()).toBeVisible();
  35  | 	await expect(page.getByRole('tab', { name: 'Source types' })).toBeVisible();
  36  | });
  37  | 
  38  | test('the list and table views mirror the legacy displays', async ({ page }) => {
  39  | 	// A category page: the list gets sortable attribute columns.
  40  | 	await page.goto('/all-modules/type/abyssal-stasis-webifier');
  41  | 	// The view buttons need hydration, which lags networkidle under
  42  | 	// parallel load on the dev server — click until the switch takes.
  43  | 	await expect(async () => {
  44  | 		await page.getByLabel('List view').first().click();
  45  | 		await expect(page.locator('.grid-cols-subgrid').first()).toBeVisible({ timeout: 1000 });
  46  | 	}).toPass();
  47  | 
  48  | 	// The table view: real table rows with the Options dropdown.
  49  | 	await expect(async () => {
  50  | 		await page.getByLabel('Table view').first().click();
  51  | 		await expect(page.locator('table')).toBeVisible({ timeout: 1000 });
  52  | 	}).toPass();
  53  | 	await expect(page.getByRole('button', { name: 'Options' }).first()).toBeVisible();
  54  | 
  55  | 	// Without a category the table has no columns to offer. The view
  56  | 	// choice persists through a background PUT, so retry the navigation
  57  | 	// until its cookie has landed.
  58  | 	await expect(async () => {
  59  | 		await page.goto('/all-modules');
  60  | 		await expect(page.getByText('Please select a category')).toBeVisible({ timeout: 1500 });
> 61  | 	}).toPass();
      |     ^ Error: expect(locator).toBeVisible() failed
  62  | 
  63  | 	// The list still works without columns: rows flow their own attributes.
  64  | 	await expect(async () => {
  65  | 		await page.getByLabel('List view').first().click();
  66  | 		await expect(page.locator('.grid-cols-subgrid').first()).toBeVisible({ timeout: 1000 });
  67  | 	}).toPass();
  68  | 
  69  | 	// Back to the grid for the other tests (the choice persists by cookie).
  70  | 	await expect(async () => {
  71  | 		await page.getByLabel('Grid view').first().click();
  72  | 		await expect(page.locator('.grid-cols-subgrid')).toHaveCount(0, { timeout: 1000 });
  73  | 	}).toPass();
  74  | });
  75  | 
  76  | test('the appraise page validates and rejects a bad link', async ({ page }) => {
  77  | 	await page.goto('/modules/add');
  78  | 	await expect(page.getByRole('heading', { name: 'Paste an item link' })).toBeVisible();
  79  | 	const appraise = page.getByRole('button', { name: 'Appraise' });
  80  | 	await expect(appraise).toBeDisabled();
  81  | 
  82  | 	// A syntactically valid link to a nonexistent item fails with the
  83  | 	// legacy notification text.
  84  | 	await page.waitForLoadState('networkidle');
  85  | 	await page.getByPlaceholder(/showinfo/).fill('<url=showinfo:47740//1>Bogus</url>');
  86  | 	await expect(appraise).toBeEnabled();
  87  | 	await appraise.click();
  88  | 	// The failure path calls real ESI from the dev stack; allow retries.
  89  | 	await expect(page.getByText('We were unable to add the module')).toBeVisible({ timeout: 20000 });
  90  | });
  91  | 
  92  | test('collections can be created through the dialog and deleted', async ({ page, baseURL }) => {
  93  | 	// A session for a character-owning user (create binds the active
  94  | 	// character).
  95  | 	const { execSync } = await import('node:child_process');
  96  | 	const { randomBytes } = await import('node:crypto');
  97  | 	const psql = (sql: string) =>
  98  | 		execSync(
  99  | 			`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(sql.replace(/\s+/g, ' ').trim())}`,
  100 | 			{ encoding: 'utf8' }
  101 | 		).trim();
  102 | 	const userId = psql('select user_id from characters where user_id is not null order by id limit 1');
  103 | 	const token = randomBytes(24).toString('hex');
  104 | 	psql(
  105 | 		`insert into sessions (token, user_id, expires_at) values ('${token}', ${userId}, now() + interval '1 hour')`
  106 | 	);
  107 | 	psql(`delete from collections where name = 'E2E Prized Rolls'`);
  108 | 	await page.context().addCookies([
  109 | 		{ name: 'mm_session', value: token, url: baseURL ?? 'http://localhost:5100' }
  110 | 	]);
  111 | 
  112 | 	await page.goto('/collections');
  113 | 	await page.waitForLoadState('networkidle');
  114 | 	await page.getByRole('button', { name: 'Create Collection' }).click();
  115 | 	await page.getByLabel('Name').fill('E2E Prized Rolls');
  116 | 	await page.getByRole('button', { name: 'Create Collection' }).last().click();
  117 | 	await expect(page).toHaveURL(/\/collections\/e2e-prized-rolls-/);
  118 | 
  119 | 	// Back on the index it sits in the personal section with the delete
  120 | 	// action; deleting removes it.
  121 | 	await page.goto('/collections');
  122 | 	await page.waitForLoadState('networkidle');
  123 | 	const card = page.locator('div').filter({ hasText: /^E2E Prized Rolls/ }).last();
  124 | 	await page.getByTitle('Delete collection').first().click();
  125 | 	await page.getByRole('button', { name: 'Delete', exact: true }).click();
  126 | 	await expect(page.getByText('E2E Prized Rolls')).toHaveCount(0);
  127 | 	void card;
  128 | });
  129 | 
  130 | test('the sell page shows the published set and the select dialog', async ({ page, baseURL }) => {
  131 | 	const { execSync } = await import('node:child_process');
  132 | 	const { randomBytes } = await import('node:crypto');
  133 | 	const psql = (sql: string) =>
  134 | 		execSync(
  135 | 			`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(sql.replace(/\s+/g, ' ').trim())}`,
  136 | 			{ encoding: 'utf8' }
  137 | 		).trim();
  138 | 	const userId = psql('select user_id from characters where user_id is not null order by id limit 1');
  139 | 	const token = randomBytes(24).toString('hex');
  140 | 	psql(
  141 | 		`insert into sessions (token, user_id, expires_at) values ('${token}', ${userId}, now() + interval '1 hour')`
  142 | 	);
  143 | 	await page.context().addCookies([
  144 | 		{ name: 'mm_session', value: token, url: baseURL ?? 'http://localhost:5100' }
  145 | 	]);
  146 | 
  147 | 	await page.goto('/sell/modules');
  148 | 	await expect(page.getByRole('heading', { name: 'Sell Modules' })).toBeVisible();
  149 | 	// Retry the click: it can land before hydration and get lost.
  150 | 	await expect(async () => {
  151 | 		await page.getByRole('button', { name: 'Select modules' }).click();
  152 | 		await expect(page.getByText(/make whole containers public/)).toBeVisible({ timeout: 1000 });
  153 | 	}).toPass();
  154 | });
  155 | 
  156 | test('guests are sent to login from the sell page', async ({ page }) => {
  157 | 	await page.goto('/sell/modules');
  158 | 	await expect(page).toHaveURL(/\/login/);
  159 | });
  160 | 
  161 | test('the offers index renders for a signed-in user', async ({ page, baseURL }) => {
```
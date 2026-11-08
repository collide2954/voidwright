# Motion

This document gives the reference values for the motion of figures. The values come from studies of human motion. The reference figure is 1.76 m tall, and its hips are 0.88 m above the ground.

In the gait tables, 0% is the heel strike of the reference leg.

## Walking at 1.67 m/s

| Item | Value | Source |
| --- | --- | --- |
| Steps | 120 to 125 steps each minute | Hirasaki 1999; Murray 1967 |
| Stride | 1.6 m | From the steps and the speed |
| Stance | 59% of the cycle | Perry and Burnfield |
| Double support | 0% to 9% and 50% to 59% | Perry and Burnfield |
| Step width | 0.11 m | Donelan 2001 |
| Foot at heel strike | 24° to 28° toes up | Fang 2018 |
| Foot at toe-off | 68° to 70° heel up | Fang 2018 |
| Thigh to vertical | +20° at heel strike, −20° at toe-off | Perry and Burnfield |
| Knee | 0° to 5° at heel strike, 15° to 20° at 12% to 15%, 40° at toe-off, 60° to 65° at 70% to 73% | Perry and Burnfield; Winter |
| Toe clearance | 1.3 cm at 80% to 85% | Winter 1992 |
| Pelvis turn | 10° to 14° in total. Each side is most forward at its own heel strike. | Chung 2010 |
| Pelvis drop | 8° in total. The swing side is lowest at 10% to 15%. | Smith 2002; Reininga 2012 |
| Body rise and fall | 4.8 cm. Lowest at 5% and 55%, highest at 30% and 80%. | Orendurff 2004 |
| Body side sway | 3.9 cm, over the stance foot | Orendurff 2004 |
| Trunk lean | 0° | Chung 2010 |
| Trunk side bend | 5° | Reininga 2012 |
| Arm swing | 8° forward to 24° back. The arm is most forward at the heel strike of the opposite leg. | Murray 1967 |
| Elbow | 17° to 47° | Murray 1967 |
| Head | The head moves with the body. It tilts 2° to 4° to keep the gaze on a point about 1 m ahead. | Pozzo 1990 |

## Running at 3.33 m/s

| Item | Value | Source |
| --- | --- | --- |
| Steps | 168 steps each minute | Fukuchi 2017 |
| Stride | 2.4 m | Fukuchi 2017 |
| Stance | 35% of the cycle | Malisoux 2023 |
| Knee | 40° to 45° in stance, 105° in swing | Novacheck 1998; Fukuchi 2017 |
| Body rise and fall | 8 cm | Malisoux 2023 |
| Trunk lean | 7° forward | Teng and Powers 2014 |

The game does not use the measured rise and fall of the run. If the body rises and falls 8 cm, the knee bends more than 45° in stance, or the foot is not on the ground at the start and at the end of each step. The game uses 5.2 cm.

## Leaning

| Case | Lean |
| --- | --- |
| Speed change | atan(a / g). For example, 11.5° at 2 m/s². |
| Turn | atan(v² / (r g)). For example, 8° when walking on a 2 m radius. |
| Start | 3° to 6°, for about 0.5 s before the first step |
| Stop | The last step is shorter. The body leans back by atan(a / g). |

Sources: Imai 2001; Farinelli 2021; Brenière and Do 1986; Jaeger 1992; Tirosh 2004.

Paths on the tile grid turn at tight corners. At these corners, the turn formula gives a large lean. The game limits the turn lean to 4°.

## Standing

| Item | Value | Source |
| --- | --- | --- |
| Sway | 0.2° to 0.5° at 0.1 Hz to 0.3 Hz | Błaszczyk and Beck 2023 |
| Breathing | 12 to 20 breaths each minute. The upper ribs move 3 mm to 5 mm. | De Groote 1997 |
| Weight shift | Small shifts about every 50 s. Larger shifts every 3 to 5 minutes. | Duarte and Zatsiorsky 1999 |
| Head turns | 10° to 45°, in 0.3 s to 0.8 s, every 5 s to 15 s (estimate) | Stahl 1999 |

Games show standing figures to the player for short times, so the game makes weight shifts more frequent than the measured values.

## Sources

- Perry and Burnfield, *Gait Analysis*, 2nd edition, 2010.
- Winter, *Biomechanics and Motor Control of Human Movement*, 4th edition, 2009.
- Murray 1967, doi:10.1093/ptj/47.4.272. Winter 1992, doi:10.1093/ptj/72.1.45. Fang 2018, doi:10.1098/rsos.170818.
- Hirasaki 1999, doi:10.1007/s002210050781. Pozzo 1990, doi:10.1007/BF00230842. Orendurff 2004, doi:10.1682/JRRD.2003.10.0150.
- Smith 2002, doi:10.1089/15246090260137626. Chung 2010, doi:10.1186/1743-0003-7-9. Reininga 2012, doi:10.1186/1743-0003-9-3.
- Donelan 2001, doi:10.1098/rspb.2001.1761. Fukuchi 2017, doi:10.7717/peerj.3298. Malisoux 2023, doi:10.1177/23259671231204629.
- Novacheck 1998, doi:10.1016/S0966-6362(97)00038-6. Teng and Powers 2014, PubMed 25155651. Imai 2001, doi:10.1007/s002210000533.
- Farinelli 2021, doi:10.3389/fnhum.2021.709780. Brenière and Do 1986, doi:10.1016/0021-9290(86)90120-X. Jaeger 1992, doi:10.1016/0021-9290(92)90080-K.
- Tirosh 2004, doi:10.1016/S0966-6362(03)00063-8. Duarte and Zatsiorsky 1999, doi:10.1123/mcj.3.1.12. Błaszczyk and Beck 2023, doi:10.5114/jhk/159452.
- De Groote 1997, doi:10.1152/jappl.1997.83.5.1531. Stahl 1999, doi:10.1007/s002210050715.

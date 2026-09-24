# Sources for korea-1950-53-unit-reference.md

This folder is git-ignored (`.gitignore` = `*`). The PDFs are the originals and the `.txt` files hold their extracted text.
In every `.txt`, pages are separated by a form feed (`\f`). PDF page N is the text after the (N-1)th `\f`.

Text method: `pdftotext -layout` where the PDF has a text layer (archive.org OCR layer or born-digital). Where it did not (S7, S9), the text comes from Windows built-in OCR (Windows.Media.Ocr), which has no layout preservation, so tables may be scrambled.

| ID | Title | Edition / date | Origin URL | Text file | PDF pages | Text method |
|---|---|---|---|---|---|---|
| S1 | FM 7-40, *Infantry Regiment* | Dept. of the Army, January 1950 | https://archive.org/details/KoreanWarFieldManuals | S1_FM7-40_Infantry_Regiment.txt | 435 | pdftotext (archive.org OCR layer) |
| S2 | FM 7-10, *Rifle Company, Infantry Regiment* | Dept. of the Army, October 1949 | https://archive.org/details/KoreanWarFieldManuals | S2_FM7-10_Rifle_Company.txt | 630 | pdftotext (archive.org OCR layer) |
| S3 | FM 17-33, *Tank Battalion* | Dept. of the Army, September 1949 | https://archive.org/details/KoreanWarFieldManuals | S3_FM17-33_Tank_Battalion.txt | 533 | pdftotext (archive.org OCR layer) |
| S4 | **FM 6-120, *The Field Artillery Observation Battalion and Batteries*.** archive.org mislabels this file as "FM 6-140 (The Field Artillery Battery)"; the title page reads FM 6-120. | Dept. of the Army, July 1951 | https://archive.org/details/KoreanWarFieldManuals | S4_FM6-140_FA_Battery.txt | 538 | pdftotext (archive.org OCR layer) |
| S5 | FM 44-2, *Antiaircraft Artillery Automatic Weapons* | Dept. of the Army, August 1950 | https://archive.org/details/KoreanWarFieldManuals | S5_FM44-2_AAA_AW.txt | 420 | pdftotext (archive.org OCR layer) |
| S6 | DA Pamphlet 30-51, *Handbook on the Chinese Communist Army* | Dept. of the Army, **7 December 1960** edition. It supersedes the 30 September 1952 edition (p.4). Its organization charts show the Chinese army after the 1955 reorganization, not Korean War tables of organization. | https://upload.wikimedia.org/wikipedia/commons/7/74/DA_Pamphlet_30-51_Handbook_on_the_Chinese_Communist_Army_7_Dece.pdf | S6_DAPam30-51_CCF_Handbook.txt | 133 | pdftotext |
| S7 | *Korean War: U.S. Pacific Fleet Operations*, CINCPACFLT Interim Evaluation Report No. 1, Vol. I (Main Report) | Covers 25 June–15 November 1950 | https://www.history.navy.mil/content/dam/nhhc/research/library/online-reading-room/war-and-conflict/korean-war/korean-war-interim-evalution/Korean-War-U.S.-Pacific-Fleet-Operations-Interim-Evaluation-Report-No-1-Volume-1%20-%2025%20June%20to%2015%20November%201950.pdf | S7_PACFLT_IER1_Vol1.txt | 28 | Windows OCR (scan, no text layer) |
| S9 | Robert F. Futrell, *The United States Air Force in Korea, 1950–1953* | Office of Air Force History, rev. ed. 1983 | https://media.defense.gov/2010/Dec/02/2001329903/-1/-1/0/usaf_in_korea-2.pdf | S9_Futrell_USAF_in_Korea.txt | 845 | Windows OCR (scan, no text layer) |
| S10 | *The U.S. Air Force in Korea: Campaigns, Units, and Stations, 1950–1953* | Air Force History and Museums Program | https://media.defense.gov/2025/Jun/12/2003737607/-1/-1/0/KOREA_CAMPAIGNS.PDF | S10_USAF_Korea_Campaigns_Units_Stations.txt | 187 | pdftotext |

Not available (listed so gaps can be attributed):
- S8, the Soviet 64th Fighter Aviation Corps report of July 1953 (Wilson Center Digital Archive), could not be fetched because the host did not resolve from this machine.
- The DTIC studies (ADA177788 *MiG Operations in Korea*, ADA440091 *MiG Alley*, ADA340611 *No More Bad Force Myths*) were unavailable.

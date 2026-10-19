# 六部位造型资料与实现边界

2026-10-19。为先打通组合流程，现有少量选项是工程起点，不是穷尽分类。
参考图中表情、视角、头发遮挡必须单独记录，不能直接当中性三维几何。

## 眼型及睫毛：复用既有研究

既有研究覆盖8款游戏12个角色，包括原神、星穹铁道、绝区零、碧蓝幻想 Relink、
鸣潮、战双帕弥什、罪恶装备 Strive、异度神剑3。目录与当前8种工作眼型见
[eye_presets](../eye_presets/README.md)。既有研究观察到高宽比、上下缘曲率、
角点高差和收束方式是相互独立的维度；不是统一椭圆的缩放。

[VRoid官方脸部编辑说明](https://vroid.pixiv.help/hc/en-us/articles/4406164546969-How-to-edit-faces)
将眼睑、眼线、睫毛、虹膜和表情职责分开，可作为参数分层参考，未提出固定8类的结论。

## 眉毛：3个研究起点

- 细直眉：纤细、主线近直，尾端收细。现有 arch/thickness 可做初版；独立长度/尾部渐尖尚需扩展。
- 柔弧眉：连续浅弧，眉头圆润、眉尾细。当前 soft-level/soft-arched 是此方向的两个工程设置，不应算两种已证明独立的大类。
- 折锋眉：主段较直，在指定峰点向尾部转折。当前 arch 单参数不能独立表达峰位和折转锐度，尚未实现。

参考：[菲伦官方角色页](https://frieren-anime.jp/character/chara_group1/1-5/)、
[芙莉莲官方角色页](https://frieren-anime.jp/character/chara_group1/1-1/)、
[Ky官方插画](https://www.guiltygear.com/ggst/en/character/kyk/)、
[Millia官方插画](https://www.guiltygear.com/ggst/en/character/mll/)。
Ky有头发遮挡，折锋完整形状包含我们的设计推演。眉毛与贴眼缘睫毛不是同一部件。

## 嘴：3个中性轮廓起点

- 小型细缝：短、薄、低唇体浮雕，参考菲伦侧面和斜侧面。
- 横向薄唇：较宽的闭合缝线，唇体较薄，参考Ky插画；去除原图微笑的嘴角姿态。
- 小型弓唇：较小宽度、明确中央上唇轮廓与下唇体积，参考Millia；去除原图张口姿态。

当前 retained、wide-neutral 是宽度变化；soft-upper-transition 只改过渡，不是完整第三类。
弓唇仍需中央轮廓、上下唇丰满度和缝线宽度的独立控制。
开口、微笑、皱眉属于表情，不以静态家族替代。张嘴仍必须保留真实口腔。

## 鼻、耳与脸型

鼻参考：[修塔尔克](https://frieren-anime.jp/character/chara_group1/1-6/)的紧凑鼻部与
[海塔](https://frieren-anime.jp/character/chara_group1/1-3/)的明确鼻梁。
当前 defined-bridge 增强鼻梁肩部，不代表完整长鼻模型或原角色复刻。

耳参考海塔的人耳与芙莉莲的长尖精灵耳。当前 compact-human 仅收敛人耳外展，
精灵耳需要独立的远端轮廓与薄壳设计，尚未实现；不能把人耳拉长等同于实现精灵耳。

脸型仍是保留轮廓加4个局部宽度变化，尚需独立的轮廓家族研究。
角色身份、性别和年龄不应由某个预设强制绑定。

## 一手技术依据与用途

Junya Motomura／Arc System Works，GDC 2015：
[GuiltyGear Xrd's Art Style](https://www.ggxrd.com/Motomura_Junya_GuiltyGearXrd.pdf)。
已检查讲义文字与第21页图示：脸部法线可为卡通光照编辑；内部线条和外轮廓需分别处理；
面部位置与形变要服务于目标二维观感。它支持固定相机/光照下的正面、斜侧、侧面对比，
不证明上述类别数量或具体造型必然美观。

部位级论文关联、更多游戏/动漫样本和跨组合验收仍在后续清单中。
保存来源与结论边界，优先让研究定义能对应独立参数和实际样例。
